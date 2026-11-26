//! The points services — the per-user points ledger with the
//! check-in/exchange faces, and the member-tier cards (the reference's
//! points_service.go / member_tier_service.go). The balance reads off
//! the latest ledger row's balance_after; the check-in awards the
//! daily points through the same append path (one row per day is the
//! dedup guarantee).

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::points_ledgers;
use store::paging::fetch_paged;

use shop_proto::proto::catalog::service::v1 as catalogv1;
use shop_proto::proto::pagination::PagingRequest;

fn ledger_proto(r: points_ledgers::Model) -> catalogv1::PointsLedger {
    catalogv1::PointsLedger {
        id: Some(r.id),
        user_id: Some(r.user_id),
        delta: Some(r.delta),
        balance_after: Some(r.balance_after),
        r#type: Some(
            catalogv1::PointsLedgerType::try_from(match r.r#type.as_str() {
                "POINTS_TYPE_DEDUCT" => 2,
                "POINTS_TYPE_CHECK_IN" => 3,
                _ => 1,
            })
            .unwrap_or(catalogv1::PointsLedgerType::PointsTypeEarn) as i32,
        ),
        reason: r.reason.clone(),
        ref_order_id: r.ref_order_id,
        created_at: r.created_at.and_then(ts_to_proto),
    }
}

/// Appends one ledger row (delta signed; the balance carries forward
/// from the user's latest row).
async fn append_ledger(
    state: &AppState,
    user_id: i64,
    delta: i64,
    type_str: &str,
    reason: &str,
    ref_order_id: Option<i64>,
) -> Result<i64, Status> {
    let latest = points_ledgers::Entity::find()
        .filter(points_ledgers::Column::UserId.eq(user_id))
        .all(&state.db)
        .await
        .map_err(db_status)?
        .into_iter()
        .map(|r| r.balance_after)
        .max()
        .unwrap_or(0);
    let balance = latest + delta;
    let active = points_ledgers::ActiveModel {
        user_id: Set(user_id),
        delta: Set(delta),
        balance_after: Set(balance),
        r#type: Set(type_str.to_string()),
        reason: Set(Some(reason.to_string())),
        ref_order_id: Set(ref_order_id),
        created_at: Set(Some(store::now())),
        ..Default::default()
    };
    active.insert(&state.db).await.map_err(db_status)?;
    Ok(balance)
}

pub struct PointsLedgerService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl catalogv1::points_ledger_service_server::PointsLedgerService for PointsLedgerService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::ListPointsLedgerResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            points_ledgers::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::ListPointsLedgerResponse {
            items: rows.into_iter().map(ledger_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::CountPointsLedgerResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            points_ledgers::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::CountPointsLedgerResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<catalogv1::GetPointsLedgerRequest>,
    ) -> Result<Response<catalogv1::PointsLedger>, Status> {
        let req = request.into_inner();
        let id = req.id;
        let row = points_ledgers::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("points ledger row not found"))?;
        Ok(Response::new(ledger_proto(row)))
    }

    /// The balance: the user's latest ledger row's balance_after.
    async fn get_user_points_balance(
        &self,
        request: Request<catalogv1::GetUserPointsBalanceRequest>,
    ) -> Result<Response<catalogv1::GetUserPointsBalanceResponse>, Status> {
        let req = request.into_inner();
        let user_id = req.user_id;
        let latest = points_ledgers::Entity::find()
            .filter(points_ledgers::Column::UserId.eq(user_id))
            .all(&self.state.db)
            .await
            .map_err(db_status)?
            .into_iter()
            .map(|r| r.balance_after)
            .max()
            .unwrap_or(0);
        Ok(Response::new(catalogv1::GetUserPointsBalanceResponse {
            balance: latest,
        }))
    }

    /// The daily check-in: one POINTS_TYPE_CHECK_IN row per UTC day
    /// (the dedup), +5 points per the reference's daily award.
    async fn check_in(
        &self,
        request: Request<catalogv1::CheckInRequest>,
    ) -> Result<Response<catalogv1::CheckInResponse>, Status> {
        let req = request.into_inner();
        let user_id = req.user_id;
        let today = store::now().format("%Y-%m-%d").to_string();
        let day_prefix = format!("daily check-in {today}");
        let existing = points_ledgers::Entity::find()
            .filter(points_ledgers::Column::UserId.eq(user_id))
            .filter(points_ledgers::Column::Type.eq("POINTS_TYPE_CHECK_IN"))
            .all(&self.state.db)
            .await
            .map_err(db_status)?
            .into_iter()
            .any(|r| {
                r.reason
                    .as_deref()
                    .map(|s| s.starts_with(&day_prefix))
                    .unwrap_or(false)
            });
        if existing {
            let balance = points_ledgers::Entity::find()
                .filter(points_ledgers::Column::UserId.eq(user_id))
                .all(&self.state.db)
                .await
                .map_err(db_status)?
                .into_iter()
                .map(|r| r.balance_after)
                .max()
                .unwrap_or(0);
            return Ok(Response::new(catalogv1::CheckInResponse {
                awarded: 0,
                balance,
                already_checked_in: true,
            }));
        }
        let balance = append_ledger(
            &self.state,
            user_id,
            5,
            "POINTS_TYPE_CHECK_IN",
            &format!("daily check-in {today}"),
            None,
        )
        .await?;
        Ok(Response::new(catalogv1::CheckInResponse {
            awarded: 5,
            balance,
            already_checked_in: false,
        }))
    }

    async fn get_check_in_status(
        &self,
        request: Request<catalogv1::GetCheckInStatusRequest>,
    ) -> Result<Response<catalogv1::GetCheckInStatusResponse>, Status> {
        let req = request.into_inner();
        let user_id = req.user_id;
        let today = store::now().format("%Y-%m-%d").to_string();
        let day_prefix = format!("daily check-in {today}");
        let rows = points_ledgers::Entity::find()
            .filter(points_ledgers::Column::UserId.eq(user_id))
            .filter(points_ledgers::Column::Type.eq("POINTS_TYPE_CHECK_IN"))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let checked_in_today = rows.iter().any(|r| {
            r.reason
                .as_deref()
                .map(|s| s.starts_with(&day_prefix))
                .unwrap_or(false)
        });
        Ok(Response::new(catalogv1::GetCheckInStatusResponse {
            checked_in_today,
            consecutive_days: rows.len() as i32,
        }))
    }

    /// The exchange face: deduct the template's points price and mint
    /// the UNUSED user coupon off the template.
    async fn exchange_points_for_coupon(
        &self,
        request: Request<catalogv1::ExchangePointsForCouponRequest>,
    ) -> Result<Response<catalogv1::ExchangePointsForCouponResponse>, Status> {
        use store::entities::{coupon_templates, user_coupons};
        let req = request.into_inner();
        let user_id = req.user_id;
        let template_id = req.coupon_template_id;
        let template = coupon_templates::Entity::find_by_id(template_id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("coupon template not found"))?;
        let price = template.points_price.unwrap_or(0);
        let balance = points_ledgers::Entity::find()
            .filter(points_ledgers::Column::UserId.eq(user_id))
            .all(&self.state.db)
            .await
            .map_err(db_status)?
            .into_iter()
            .map(|r| r.balance_after)
            .max()
            .unwrap_or(0);
        if balance < price {
            return Err(bad("insufficient points"));
        }
        let new_balance = append_ledger(
            &self.state,
            user_id,
            -price,
            "POINTS_TYPE_DEDUCT",
            &format!("exchange coupon template {template_id}"),
            None,
        )
        .await?;
        let active = user_coupons::ActiveModel {
            user_id: Set(user_id),
            coupon_template_id: Set(template_id),
            status: Set("UNUSED".to_string()),
            tenant_id: Set(template.tenant_id),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        let coupon = active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(catalogv1::ExchangePointsForCouponResponse {
            coupon_id: coupon.id,
            points_deducted: price,
            balance: new_balance,
        }))
    }
}
