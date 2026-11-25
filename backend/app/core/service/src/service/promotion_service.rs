//! The promotion service — the per-product special prices and the
//! shop-wide full reductions (the reference's promotion_service.go).
//! The storefront's ListActivePromotions (whitelisted anonymous face)
//! walks the ACTIVE rows inside their window.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_from_proto, ts_to_proto, AppState};
use store::entities::promotions;
use store::paging::fetch_paged;

use shop_proto::proto::catalog::service::v1 as catalogv1;
use shop_proto::proto::pagination::PagingRequest;

fn promo_type_to_str(v: i32) -> &'static str {
    use catalogv1::promotion::PromoType;
    match catalogv1::promotion::PromoType::try_from(v).ok() {
        Some(PromoType::FullReduction) => "FULL_REDUCTION",
        _ => "SPECIAL_PRICE",
    }
}

fn promotion_proto(r: promotions::Model) -> catalogv1::Promotion {
    catalogv1::Promotion {
        id: Some(r.id),
        name: Some(r.name.clone()),
        promo_type: Some(
            catalogv1::promotion::PromoType::try_from(match r.promo_type.as_str() {
                "FULL_REDUCTION" => 2,
                _ => 1,
            })
            .unwrap_or(catalogv1::promotion::PromoType::SpecialPrice) as i32,
        ),
        product_id: r.product_id,
        special_price: Some(r.special_price.unwrap_or(0)),
        threshold_amount: Some(r.threshold_amount.unwrap_or(0)),
        reduction_amount: Some(r.reduction_amount.unwrap_or(0)),
        priority: r.priority,
        valid_from: r.valid_from.and_then(ts_to_proto),
        valid_until: r.valid_until.and_then(ts_to_proto),
        currency: Some(r.currency.clone()),
        status: Some(
            if r.status == "ACTIVE" {
                catalogv1::promotion::Status::Active
            } else {
                catalogv1::promotion::Status::Inactive
            } as i32,
        ),
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

pub struct PromotionService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl catalogv1::promotion_service_server::PromotionService for PromotionService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::ListPromotionResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            promotions::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::ListPromotionResponse {
            items: rows.into_iter().map(promotion_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::CountPromotionResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            promotions::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::CountPromotionResponse { count: total as i64 }))
    }

    async fn get(
        &self,
        request: Request<catalogv1::GetPromotionRequest>,
    ) -> Result<Response<catalogv1::Promotion>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::get_promotion_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = promotions::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("promotion not found"))?;
        Ok(Response::new(promotion_proto(row)))
    }

    async fn create(
        &self,
        request: Request<catalogv1::CreatePromotionRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let name = data
            .name
            .clone()
            .filter(|n| !n.is_empty())
            .ok_or_else(|| bad("data.name required"))?;
        let active = promotions::ActiveModel {
            name: Set(name),
            promo_type: Set(promo_type_to_str(data.promo_type.unwrap_or(1)).to_string()),
            product_id: Set(data.product_id),
            special_price: Set(Some(data.special_price.unwrap_or(0))),
            threshold_amount: Set(Some(data.threshold_amount.unwrap_or(0))),
            reduction_amount: Set(Some(data.reduction_amount.unwrap_or(0))),
            priority: Set(data.priority.map(|v| v as i32)),
            valid_from: Set(data.valid_from.map(ts_from_proto)),
            valid_until: Set(data.valid_until.map(ts_from_proto)),
            currency: Set(data.currency.unwrap_or_else(|| "CNY".to_string())),
            status: Set(
                if data.status == Some(catalogv1::promotion::Status::Inactive as i32) {
                    "INACTIVE"
                } else {
                    "ACTIVE"
                }
                .to_string(),
            ),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<catalogv1::UpdatePromotionRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = promotions::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("promotion not found"))?;
        let mut active: promotions::ActiveModel = existing.into();
        if let Some(v) = data.name.clone() {
            active.name = Set(v);
        }
        if let Some(v) = data.special_price {
            active.special_price = Set(Some(v));
        }
        if let Some(v) = data.reduction_amount {
            active.reduction_amount = Set(Some(v));
        }
        if let Some(v) = data.status {
            active.status = Set(
                if v == catalogv1::promotion::Status::Active as i32 {
                    "ACTIVE"
                } else {
                    "INACTIVE"
                }
                .to_string(),
            );
        }
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<catalogv1::DeletePromotionRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::delete_promotion_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        promotions::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The storefront anonymous face: the ACTIVE promotions inside
    /// their validity window (the product filter stays client-side per
    /// the contract's comment).
    async fn list_active_promotions(
        &self,
        request: Request<catalogv1::ListActivePromotionsRequest>,
    ) -> Result<Response<catalogv1::ListPromotionResponse>, Status> {
        let _ = request.into_inner();
        let rows = promotions::Entity::find()
            .filter(promotions::Column::Status.eq("ACTIVE"))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let now: chrono::DateTime<chrono::FixedOffset> =
            chrono::DateTime::from(store::now());
        let items: Vec<catalogv1::Promotion> = rows
            .into_iter()
            .filter(|r| {
                r.valid_from.map(|v| v <= now).unwrap_or(true)
                    && r.valid_until.map(|v| v >= now).unwrap_or(true)
            })
            .map(promotion_proto)
            .collect();
        let total = items.len() as i64;
        Ok(Response::new(catalogv1::ListPromotionResponse {
            items,
            total,
        }))
    }
}
