//! The coupon services — the templates and the user-held instances
//! (the reference's coupon_template_service.go /
//! user_coupon_service.go). Claim enforces the redemption caps
//! (template-wide and per-user); Quote walks the discount math
//! (FIXED_AMOUNT face value; PERCENTAGE joins the caller's total with
//! the cart phase).

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{coupon_templates, user_coupons};
use store::paging::fetch_paged;

use shop_proto::proto::coupon::service::v1 as couponv1;
use shop_proto::proto::pagination::PagingRequest;

/// protojson Timestamp → the entity's timestamptz.
fn ts_from_proto(v: pbjson_types::Timestamp) -> sea_orm::prelude::DateTimeWithTimeZone {
    use chrono::TimeZone as _;
    chrono::Utc
        .timestamp_opt(v.seconds, v.nanos.max(0) as u32)
        .single()
        .unwrap_or_else(chrono::Utc::now)
        .fixed_offset()
}

/// The enum ↔ stored spellings (the demo data pins the bare member
/// names: FIXED_AMOUNT, ACTIVE, UNUSED, USED, ...).
fn discount_type_to_str(v: i32) -> Option<String> {
    use couponv1::coupon_template::DiscountType;
    Some(match DiscountType::try_from(v).ok()? {
        DiscountType::FixedAmount => "FIXED_AMOUNT".to_string(),
        DiscountType::Percentage => "PERCENTAGE".to_string(),
        _ => return None,
    })
}

#[allow(dead_code)]
fn discount_type_to_i32(stored: &str) -> Option<i32> {
    use couponv1::coupon_template::DiscountType;
    Some(match stored {
        "FIXED_AMOUNT" => DiscountType::FixedAmount,
        "PERCENTAGE" => DiscountType::Percentage,
        _ => return None,
    } as i32)
}

fn template_status_to_str(v: i32) -> Option<String> {
    use couponv1::coupon_template::Status;
    Some(match Status::try_from(v).ok()? {
        Status::Active => "ACTIVE".to_string(),
        Status::Inactive => Status::Inactive.as_str_name().to_string(),
        _ => return None,
    })
}

#[allow(dead_code)]
fn template_status_to_i32(stored: &str) -> Option<i32> {
    use couponv1::coupon_template::Status;
    Some(match stored {
        "ACTIVE" => Status::Active,
        "INACTIVE" => Status::Inactive,
        _ => return None,
    } as i32)
}

fn user_status_to_str(v: i32) -> Option<String> {
    use couponv1::user_coupon::Status;
    Some(match Status::try_from(v).ok()? {
        Status::Unused => "UNUSED".to_string(),
        Status::Used => "USED".to_string(),
        _ => return None,
    })
}

#[allow(dead_code)]
fn user_status_to_i32(stored: &str) -> Option<i32> {
    use couponv1::user_coupon::Status;
    Some(match stored {
        "UNUSED" => Status::Unused,
        "USED" => Status::Used,
        _ => return None,
    } as i32)
}

fn template_proto(r: coupon_templates::Model) -> couponv1::CouponTemplate {
    couponv1::CouponTemplate {
        id: Some(r.id),
        discount_type: discount_type_to_i32(&r.discount_type),
        discount_value: r.discount_value,
        discount_percentage: r.discount_percentage,
        valid_from: r.valid_from.and_then(ts_to_proto),
        valid_until: r.valid_until.and_then(ts_to_proto),
        max_redemptions: r.max_redemptions,
        max_redemptions_per_user: r.max_redemptions_per_user,
        redeemed_count: r.redeemed_count,
        status: template_status_to_i32(&r.status),
        currency: Some(r.currency.clone()),
        points_price: r.points_price,
        claimable: r.claimable,
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

fn user_coupon_proto(r: user_coupons::Model) -> couponv1::UserCoupon {
    couponv1::UserCoupon {
        id: Some(r.id),
        user_id: Some(r.user_id),
        coupon_template_id: Some(r.coupon_template_id),
        status: user_status_to_i32(&r.status),
        redeemed_at: r.redeemed_at.and_then(ts_to_proto),
        redeemed_order_id: r.redeemed_order_id,
        applied_discount_amount: r.applied_discount_amount,
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

/// The claim's operator: the forwarded x-user-id metadata (the same
/// stamp the cart write face reads).
fn operator_of<T>(request: &Request<T>) -> i64 {
    request
        .metadata()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0)
}

pub struct CouponTemplateService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl couponv1::coupon_template_service_server::CouponTemplateService for CouponTemplateService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<couponv1::ListCouponTemplateResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            coupon_templates::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(couponv1::ListCouponTemplateResponse {
            items: rows.into_iter().map(template_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<couponv1::CountCouponTemplateResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            coupon_templates::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(couponv1::CountCouponTemplateResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<couponv1::GetCouponTemplateRequest>,
    ) -> Result<Response<couponv1::CouponTemplate>, Status> {
        let req = request.into_inner();
        let Some(couponv1::get_coupon_template_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = coupon_templates::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("coupon template not found"))?;
        Ok(Response::new(template_proto(row)))
    }

    async fn create(
        &self,
        request: Request<couponv1::CreateCouponTemplateRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let active = coupon_templates::ActiveModel {
            currency: Set(data.currency.unwrap_or_else(|| "CNY".to_string())),
            discount_type: Set(data
                .discount_type
                .and_then(discount_type_to_str)
                .unwrap_or_else(|| "FIXED_AMOUNT".to_string())),
            discount_value: Set(Some(data.discount_value.unwrap_or(0))),
            discount_percentage: Set(data.discount_percentage),
            valid_from: Set(data.valid_from.map(ts_from_proto)),
            valid_until: Set(data.valid_until.map(ts_from_proto)),
            max_redemptions: Set(data.max_redemptions),
            max_redemptions_per_user: Set(data.max_redemptions_per_user),
            redeemed_count: Set(Some(0)),
            status: Set(data
                .status
                .and_then(template_status_to_str)
                .unwrap_or_else(|| "ACTIVE".to_string())),
            claimable: Set(Some(data.claimable.unwrap_or(true))),
            points_price: Set(Some(data.points_price.unwrap_or(0))),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<couponv1::UpdateCouponTemplateRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = coupon_templates::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("coupon template not found"))?;
        let mut active: coupon_templates::ActiveModel = existing.into();
        if let Some(v) = data.discount_type.and_then(discount_type_to_str) {
            active.discount_type = Set(v);
        }
        if let Some(v) = data.discount_value {
            active.discount_value = Set(Some(v));
        }
        if let Some(v) = data.status.and_then(template_status_to_str) {
            active.status = Set(v);
        }
        if let Some(v) = data.claimable {
            active.claimable = Set(Some(v));
        }
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<couponv1::DeleteCouponTemplateRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(couponv1::delete_coupon_template_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        coupon_templates::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

pub struct UserCouponService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl couponv1::user_coupon_service_server::UserCouponService for UserCouponService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<couponv1::ListUserCouponResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            user_coupons::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(couponv1::ListUserCouponResponse {
            items: rows.into_iter().map(user_coupon_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<couponv1::CountUserCouponResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            user_coupons::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(couponv1::CountUserCouponResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<couponv1::GetUserCouponRequest>,
    ) -> Result<Response<couponv1::UserCoupon>, Status> {
        let req = request.into_inner();
        let Some(couponv1::get_user_coupon_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = user_coupons::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("user coupon not found"))?;
        Ok(Response::new(user_coupon_proto(row)))
    }

    async fn create(
        &self,
        request: Request<couponv1::CreateUserCouponRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let user_id = data.user_id.ok_or_else(|| bad("data.userId required"))?;
        let template_id = data
            .coupon_template_id
            .ok_or_else(|| bad("data.couponTemplateId required"))?;
        let active = user_coupons::ActiveModel {
            user_id: Set(user_id),
            coupon_template_id: Set(template_id),
            status: Set(data
                .status
                .and_then(user_status_to_str)
                .unwrap_or_else(|| "UNUSED".to_string())),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<couponv1::UpdateUserCouponRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = user_coupons::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("user coupon not found"))?;
        let mut active: user_coupons::ActiveModel = existing.into();
        if let Some(v) = data.status.and_then(user_status_to_str) {
            active.status = Set(v);
        }
        if let Some(v) = data.applied_discount_amount {
            active.applied_discount_amount = Set(Some(v));
        }
        if let Some(v) = data.redeemed_order_id {
            active.redeemed_order_id = Set(Some(v));
        }
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<couponv1::DeleteUserCouponRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(couponv1::delete_user_coupon_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        user_coupons::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The claim face: the template's caps enforced — claimable flag,
    /// ACTIVE status, the template-wide redemption count, and the
    /// per-user cap — then the instance lands UNUSED and the
    /// template-wide counter ticks.
    async fn claim(
        &self,
        request: Request<couponv1::ClaimCouponRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let op = operator_of(&request);
        let req = request.into_inner();
        let template_id = req.coupon_template_id;
        if op == 0 {
            return Err(bad("operator identity required (x-user-id)"));
        }
        let template = coupon_templates::Entity::find_by_id(template_id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("coupon template not found"))?;
        if template.status != "ACTIVE" || template.claimable != Some(true) {
            return Err(bad("coupon template not claimable"));
        }
        if let Some(max) = template.max_redemptions {
            if template.redeemed_count.unwrap_or(0) >= max {
                return Err(bad("coupon template exhausted"));
            }
        }
        let held = user_coupons::Entity::find()
            .filter(user_coupons::Column::UserId.eq(op))
            .filter(user_coupons::Column::CouponTemplateId.eq(template_id))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let per_user = template.max_redemptions_per_user.unwrap_or(1).max(1);
        if held.len() as i32 >= per_user {
            return Err(bad("per-user redemption cap reached"));
        }
        let tenant = template.tenant_id;
        let active = user_coupons::ActiveModel {
            user_id: Set(op),
            coupon_template_id: Set(template_id),
            status: Set("UNUSED".to_string()),
            tenant_id: Set(tenant),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        let mut t_active: coupon_templates::ActiveModel = template.into();
        let redeemed = t_active.redeemed_count.clone().take().unwrap_or_default();
        t_active.redeemed_count = Set(Some(redeemed.unwrap_or(0) + 1));
        t_active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The quote face: the coupon's applicability verdict plus the
    /// discount math (FIXED_AMOUNT face value; PERCENTAGE joins the
    /// caller's pre-discount total with the cart phase — the bare
    /// quote answers the face capability).
    async fn quote(
        &self,
        request: Request<couponv1::QuoteRequest>,
    ) -> Result<Response<couponv1::QuoteResponse>, Status> {
        let req = request.into_inner();
        let user_coupon_id = req
            .user_coupon_id
            .ok_or_else(|| bad("user_coupon_id required"))?;
        let row = user_coupons::Entity::find_by_id(user_coupon_id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("user coupon not found"))?;
        let template = coupon_templates::Entity::find_by_id(row.coupon_template_id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("coupon template not found"))?;
        let applicable = row.status == "UNUSED"
            && template.status == "ACTIVE"
            && template
                .valid_until
                .map(|v| v > chrono::DateTime::<chrono::FixedOffset>::from(store::now()))
                .unwrap_or(true);
        let discount = if applicable && template.discount_type == "FIXED_AMOUNT" {
            template.discount_value.unwrap_or(0)
        } else {
            0
        };
        let pre = 0i64;
        Ok(Response::new(couponv1::QuoteResponse {
            applicable: Some(applicable),
            pre_discount_total: Some(pre),
            discount: Some(discount),
            post_discount_total: Some((pre - discount).max(0)),
        }))
    }
}
