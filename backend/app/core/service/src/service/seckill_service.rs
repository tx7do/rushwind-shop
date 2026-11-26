//! The seckill service — the flash-sale activities (the reference's
//! seckill_service.go). The activity face is admin CRUD plus the
//! storefront's anonymous ListActiveSeckills; the sold-count ticks
//! ride the order flow's landing.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_from_proto, ts_to_proto, AppState};
use store::entities::seckill_activities;
use store::paging::fetch_paged;

use shop_proto::proto::catalog::service::v1 as catalogv1;
use shop_proto::proto::pagination::PagingRequest;

fn activity_proto(r: seckill_activities::Model) -> catalogv1::SeckillActivity {
    catalogv1::SeckillActivity {
        id: Some(r.id),
        name: Some(r.name.clone()),
        sku_id: Some(r.sku_id),
        seckill_price: Some(r.seckill_price),
        total_stock: Some(r.total_stock),
        sold_count: Some(r.sold_count),
        per_user_limit: r.per_user_limit,
        priority: r.priority,
        valid_from: r.valid_from.and_then(ts_to_proto),
        valid_until: r.valid_until.and_then(ts_to_proto),
        currency: Some(r.currency.clone()),
        status: Some(
            catalogv1::seckill_activity::Status::try_from(if r.status == "ACTIVE" { 1 } else { 2 })
                .unwrap_or(catalogv1::seckill_activity::Status::Active) as i32,
        ),
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

pub struct SeckillActivityService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl catalogv1::seckill_activity_service_server::SeckillActivityService for SeckillActivityService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::ListSeckillActivityResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            seckill_activities::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::ListSeckillActivityResponse {
            items: rows.into_iter().map(activity_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::CountSeckillActivityResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            seckill_activities::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::CountSeckillActivityResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<catalogv1::GetSeckillActivityRequest>,
    ) -> Result<Response<catalogv1::SeckillActivity>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::get_seckill_activity_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = seckill_activities::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("seckill activity not found"))?;
        Ok(Response::new(activity_proto(row)))
    }

    async fn create(
        &self,
        request: Request<catalogv1::CreateSeckillActivityRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let name = data
            .name
            .clone()
            .filter(|n| !n.is_empty())
            .ok_or_else(|| bad("data.name required"))?;
        let sku_id = data.sku_id.ok_or_else(|| bad("data.skuId required"))?;
        let active = seckill_activities::ActiveModel {
            name: Set(name),
            sku_id: Set(sku_id),
            seckill_price: Set(data.seckill_price.unwrap_or(0)),
            total_stock: Set(data.total_stock.unwrap_or(0)),
            sold_count: Set(0),
            per_user_limit: Set(data.per_user_limit),
            priority: Set(data.priority),
            valid_from: Set(data.valid_from.map(ts_from_proto)),
            valid_until: Set(data.valid_until.map(ts_from_proto)),
            currency: Set(data.currency.unwrap_or_else(|| "CNY".to_string())),
            status: Set(
                if data.status == Some(catalogv1::seckill_activity::Status::Inactive as i32) {
                    "INACTIVE"
                } else {
                    "ACTIVE"
                }
                .to_string(),
            ),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<catalogv1::UpdateSeckillActivityRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = seckill_activities::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("seckill activity not found"))?;
        let mut active: seckill_activities::ActiveModel = existing.into();
        if let Some(v) = data.name.clone() {
            active.name = Set(v);
        }
        if let Some(v) = data.seckill_price {
            active.seckill_price = Set(v);
        }
        if let Some(v) = data.total_stock {
            active.total_stock = Set(v);
        }
        if let Some(v) = data.status {
            active.status = Set(if v == catalogv1::seckill_activity::Status::Active as i32 {
                "ACTIVE"
            } else {
                "INACTIVE"
            }
            .to_string());
        }
        active.updated_at = Set(Some(store::now()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<catalogv1::DeleteSeckillActivityRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let id = req.id;
        seckill_activities::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The storefront anonymous face: the ACTIVE activities inside
    /// their validity window.
    async fn list_active_seckills(
        &self,
        request: Request<catalogv1::ListActiveSeckillsRequest>,
    ) -> Result<Response<catalogv1::ListSeckillActivityResponse>, Status> {
        let _ = request.into_inner();
        let rows = seckill_activities::Entity::find()
            .filter(seckill_activities::Column::Status.eq("ACTIVE"))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let now: chrono::DateTime<chrono::FixedOffset> = store::now();
        let items: Vec<catalogv1::SeckillActivity> = rows
            .into_iter()
            .filter(|r| {
                r.valid_from.map(|v| v <= now).unwrap_or(true)
                    && r.valid_until.map(|v| v >= now).unwrap_or(true)
            })
            .map(activity_proto)
            .collect();
        let total = items.len() as i64;
        Ok(Response::new(catalogv1::ListSeckillActivityResponse {
            items,
            total,
        }))
    }
}
