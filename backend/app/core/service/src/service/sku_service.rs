//! The sku services — the sellable unit (code + stock), its prices
//! (per currency, the amount kept as the exact string the reference
//! stores), and the attribute combinations (the reference's
//! sku_service.go / sku_price_service.go /
//! sku_attribute_combination_service.go).

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{sku_attribute_combinations, sku_prices, skus};
use store::paging::fetch_paged;

use shop_proto::proto::catalog::service::v1 as catalogv1;
use shop_proto::proto::pagination::PagingRequest;

fn sku_proto(r: skus::Model) -> catalogv1::Sku {
    catalogv1::Sku {
        id: Some(r.id),
        product_id: Some(r.product_id),
        sku_code: Some(r.sku_code),
        stock_qty: Some(r.stock_qty),
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

fn sku_price_proto(r: sku_prices::Model) -> catalogv1::SkuPrice {
    catalogv1::SkuPrice {
        id: Some(r.id),
        sku_id: Some(r.sku_id),
        currency: Some(r.currency),
        amount: Some(r.amount),
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

fn combination_proto(r: sku_attribute_combinations::Model) -> catalogv1::SkuAttributeCombination {
    catalogv1::SkuAttributeCombination {
        id: Some(r.id),
        sku_id: Some(r.sku_id),
        attribute_id: Some(r.attribute_id),
        attribute_value_id: r.attribute_value_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

// ── SkuService ──────────────────────────────────────────────────────

pub struct SkuService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl catalogv1::sku_service_server::SkuService for SkuService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::ListSkuResponse>, Status> {
        let (rows, total) =
            fetch_paged(&self.state.db, skus::Entity::find(), &request.into_inner())
                .await
                .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::ListSkuResponse {
            items: rows.into_iter().map(sku_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::CountSkuResponse>, Status> {
        let (_, total) = fetch_paged(&self.state.db, skus::Entity::find(), &request.into_inner())
            .await
            .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::CountSkuResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<catalogv1::GetSkuRequest>,
    ) -> Result<Response<catalogv1::Sku>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::get_sku_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = skus::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("sku not found"))?;
        Ok(Response::new(sku_proto(row)))
    }

    async fn create(
        &self,
        request: Request<catalogv1::CreateSkuRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let product_id = data
            .product_id
            .ok_or_else(|| bad("data.productId required"))?;
        let active = skus::ActiveModel {
            product_id: Set(product_id),
            sku_code: Set(data.sku_code.unwrap_or_default()),
            stock_qty: Set(data.stock_qty.unwrap_or(0)),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn batch_create(
        &self,
        request: Request<catalogv1::BatchCreateSkusRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        for data in req.items {
            let Some(product_id) = data.product_id else {
                return Err(bad("data.productId required"));
            };
            let active = skus::ActiveModel {
                product_id: Set(product_id),
                sku_code: Set(data.sku_code.unwrap_or_default()),
                stock_qty: Set(data.stock_qty.unwrap_or(0)),
                created_at: Set(Some(store::now())),
                ..Default::default()
            };
            active.insert(&self.state.db).await.map_err(db_status)?;
        }
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<catalogv1::UpdateSkuRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = skus::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("sku not found"))?;
        let mut active: skus::ActiveModel = existing.into();
        if let Some(v) = data.sku_code {
            active.sku_code = Set(v);
        }
        if let Some(v) = data.stock_qty {
            active.stock_qty = Set(v);
        }
        if let Some(v) = data.product_id {
            active.product_id = Set(v);
        }
        active.updated_at = Set(Some(store::now()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<catalogv1::DeleteSkuRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::delete_sku_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        skus::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn adjust_stock(
        &self,
        request: Request<catalogv1::AdjustSkuStockRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let existing = skus::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("sku not found"))?;
        let mut active: skus::ActiveModel = existing.into();
        let next = active.stock_qty.clone().take().unwrap_or(0) + req.delta;
        if next < 0 {
            return Err(bad("stock would go negative"));
        }
        active.stock_qty = Set(next);
        active.updated_at = Set(Some(store::now()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

// ── SkuPriceService ─────────────────────────────────────────────────

pub struct SkuPriceService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl catalogv1::sku_price_service_server::SkuPriceService for SkuPriceService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::ListSkuPriceResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sku_prices::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::ListSkuPriceResponse {
            items: rows.into_iter().map(sku_price_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::CountSkuResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            sku_prices::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::CountSkuResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<catalogv1::GetSkuPriceRequest>,
    ) -> Result<Response<catalogv1::SkuPrice>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::get_sku_price_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = sku_prices::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("sku price not found"))?;
        Ok(Response::new(sku_price_proto(row)))
    }

    async fn create(
        &self,
        request: Request<catalogv1::CreateSkuPriceRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let sku_id = data.sku_id.ok_or_else(|| bad("data.skuId required"))?;
        let active = sku_prices::ActiveModel {
            sku_id: Set(sku_id),
            currency: Set(data.currency.unwrap_or_else(|| "CNY".to_string())),
            amount: Set(data.amount.unwrap_or_else(|| "0".to_string())),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn batch_create(
        &self,
        request: Request<catalogv1::BatchCreateSkuPricesRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        for data in req.items {
            let Some(sku_id) = data.sku_id else {
                return Err(bad("data.skuId required"));
            };
            let active = sku_prices::ActiveModel {
                sku_id: Set(sku_id),
                currency: Set(data.currency.unwrap_or_else(|| "CNY".to_string())),
                amount: Set(data.amount.unwrap_or_else(|| "0".to_string())),
                created_at: Set(Some(store::now())),
                ..Default::default()
            };
            active.insert(&self.state.db).await.map_err(db_status)?;
        }
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<catalogv1::UpdateSkuPriceRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = sku_prices::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("sku price not found"))?;
        let mut active: sku_prices::ActiveModel = existing.into();
        if let Some(v) = data.currency {
            active.currency = Set(v);
        }
        if let Some(v) = data.amount {
            active.amount = Set(v);
        }
        active.updated_at = Set(Some(store::now()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<catalogv1::DeleteSkuPriceRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::delete_sku_price_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        sku_prices::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

// ── SkuAttributeCombinationService ──────────────────────────────────

pub struct SkuAttributeCombinationService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl catalogv1::sku_attribute_combination_service_server::SkuAttributeCombinationService
    for SkuAttributeCombinationService
{
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::ListSkuAttributeCombinationResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sku_attribute_combinations::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(
            catalogv1::ListSkuAttributeCombinationResponse {
                items: rows.into_iter().map(combination_proto).collect(),
                total: total as i64,
            },
        ))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::CountSkuAttributeCombinationResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            sku_attribute_combinations::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(
            catalogv1::CountSkuAttributeCombinationResponse {
                count: total as i64,
            },
        ))
    }

    async fn get(
        &self,
        request: Request<catalogv1::GetSkuAttributeCombinationRequest>,
    ) -> Result<Response<catalogv1::SkuAttributeCombination>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::get_sku_attribute_combination_request::QueryBy::Id(id)) = req.query_by
        else {
            return Err(bad("query_by required"));
        };
        let row = sku_attribute_combinations::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("combination not found"))?;
        Ok(Response::new(combination_proto(row)))
    }

    async fn create(
        &self,
        request: Request<catalogv1::CreateSkuAttributeCombinationRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let sku_id = data.sku_id.ok_or_else(|| bad("data.skuId required"))?;
        let attribute_id = data
            .attribute_id
            .ok_or_else(|| bad("data.attributeId required"))?;
        let active = sku_attribute_combinations::ActiveModel {
            sku_id: Set(sku_id),
            attribute_id: Set(attribute_id),
            attribute_value_id: Set(data.attribute_value_id),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn batch_create(
        &self,
        request: Request<catalogv1::BatchCreateSkuAttributeCombinationsRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        for data in req.items {
            let Some(sku_id) = data.sku_id else {
                return Err(bad("data.skuId required"));
            };
            let Some(attribute_id) = data.attribute_id else {
                return Err(bad("data.attributeId required"));
            };
            let active = sku_attribute_combinations::ActiveModel {
                sku_id: Set(sku_id),
                attribute_id: Set(attribute_id),
                attribute_value_id: Set(data.attribute_value_id),
                created_at: Set(Some(store::now())),
                ..Default::default()
            };
            active.insert(&self.state.db).await.map_err(db_status)?;
        }
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<catalogv1::UpdateSkuAttributeCombinationRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = sku_attribute_combinations::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("combination not found"))?;
        let mut active: sku_attribute_combinations::ActiveModel = existing.into();
        if let Some(v) = data.attribute_id {
            active.attribute_id = Set(v);
        }
        active.attribute_value_id = Set(data.attribute_value_id);
        active.updated_at = Set(Some(store::now()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<catalogv1::DeleteSkuAttributeCombinationRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::delete_sku_attribute_combination_request::QueryBy::Id(id)) =
            req.query_by
        else {
            return Err(bad("query_by required"));
        };
        sku_attribute_combinations::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}
