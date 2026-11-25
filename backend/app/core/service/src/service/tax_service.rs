//! The tax-rate service — the per-region tax cards (the reference's
//! tax_rate_service.go): the region/currency pair with the integer
//! percentage and the ACTIVE/INACTIVE status, the same shape the
//! shipping rates ride.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::tax_rates;
use store::paging::fetch_paged;

use shop_proto::proto::pagination::PagingRequest;
use shop_proto::proto::tax::service::v1 as taxv1;

fn rate_proto(r: tax_rates::Model) -> taxv1::TaxRate {
    taxv1::TaxRate {
        id: Some(r.id),
        region: Some(r.region.clone()),
        tax_rate: Some(r.tax_rate),
        status: Some(if r.status == "ACTIVE" {
            taxv1::tax_rate::Status::Active
        } else {
            taxv1::tax_rate::Status::Inactive
        } as i32),
        currency: Some(r.currency.clone()),
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

pub struct TaxRateService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl taxv1::tax_rate_service_server::TaxRateService for TaxRateService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<taxv1::ListTaxRateResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            tax_rates::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(taxv1::ListTaxRateResponse {
            items: rows.into_iter().map(rate_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<taxv1::CountTaxRateResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            tax_rates::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(taxv1::CountTaxRateResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<taxv1::GetTaxRateRequest>,
    ) -> Result<Response<taxv1::TaxRate>, Status> {
        let req = request.into_inner();
        let Some(taxv1::get_tax_rate_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = tax_rates::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("tax rate not found"))?;
        Ok(Response::new(rate_proto(row)))
    }

    async fn create(
        &self,
        request: Request<taxv1::CreateTaxRateRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let region = data
            .region
            .clone()
            .filter(|r| !r.is_empty())
            .ok_or_else(|| bad("data.region required"))?;
        let active = tax_rates::ActiveModel {
            region: Set(region),
            tax_rate: Set(data.tax_rate.unwrap_or(0) as i32),
            currency: Set(data.currency.unwrap_or_else(|| "CNY".to_string())),
            status: Set(
                if data.status == Some(taxv1::tax_rate::Status::Inactive as i32) {
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
        request: Request<taxv1::UpdateTaxRateRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = tax_rates::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("tax rate not found"))?;
        let mut active: tax_rates::ActiveModel = existing.into();
        if let Some(v) = data.region.clone() {
            active.region = Set(v);
        }
        if let Some(v) = data.tax_rate {
            active.tax_rate = Set(v as i32);
        }
        if let Some(v) = data.currency.clone() {
            active.currency = Set(v);
        }
        if let Some(v) = data.status {
            active.status = Set(if v == taxv1::tax_rate::Status::Active as i32 {
                "ACTIVE"
            } else {
                "INACTIVE"
            }
            .to_string());
        }
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<taxv1::DeleteTaxRateRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(taxv1::delete_tax_rate_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        tax_rates::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}
