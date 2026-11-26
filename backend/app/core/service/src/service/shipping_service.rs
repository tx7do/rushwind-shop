//! The shipping services — the rate cards per region and the
//! shipments (the reference's shipping_rate_service.go /
//! shipment_service.go). The stored spellings follow the demo data:
//! the rates carry `ACTIVE`/`INACTIVE`, the shipments walk
//! PENDING → SHIPPED → DELIVERED.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{shipments, shipping_rates};
use store::paging::fetch_paged;

use shop_proto::proto::pagination::PagingRequest;
use shop_proto::proto::shipping::service::v1 as shippingv1;

fn rate_proto(r: shipping_rates::Model) -> shippingv1::ShippingRate {
    shippingv1::ShippingRate {
        id: Some(r.id),
        region: Some(r.region.clone()),
        base_fee: Some(r.base_fee),
        per_unit_fee: r.per_unit_fee,
        status: Some(
            shippingv1::shipping_rate::Status::try_from(if r.status == "ACTIVE" { 1 } else { 2 })
                .unwrap_or(shippingv1::shipping_rate::Status::Active) as i32,
        ),
        currency: Some(r.currency.clone()),
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

fn shipment_proto(r: shipments::Model) -> shippingv1::Shipment {
    shippingv1::Shipment {
        id: Some(r.id),
        order_id: Some(r.order_id),
        user_id: r.user_id,
        carrier: r.carrier.clone(),
        tracking_number: r.tracking_number.clone(),
        status: Some(
            shippingv1::shipment::Status::try_from(match r.status.as_str() {
                "SHIPPED" => 2,
                "DELIVERED" => 3,
                _ => 1,
            })
            .unwrap_or(shippingv1::shipment::Status::Pending) as i32,
        ),
        tracking_events: r.tracking_events.clone(),
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

pub struct ShippingRateService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl shippingv1::shipping_rate_service_server::ShippingRateService for ShippingRateService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<shippingv1::ListShippingRateResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            shipping_rates::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(shippingv1::ListShippingRateResponse {
            items: rows.into_iter().map(rate_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<shippingv1::CountShippingRateResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            shipping_rates::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(shippingv1::CountShippingRateResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<shippingv1::GetShippingRateRequest>,
    ) -> Result<Response<shippingv1::ShippingRate>, Status> {
        let req = request.into_inner();
        let Some(shippingv1::get_shipping_rate_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = shipping_rates::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("shipping rate not found"))?;
        Ok(Response::new(rate_proto(row)))
    }

    async fn create(
        &self,
        request: Request<shippingv1::CreateShippingRateRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let region = data.region.ok_or_else(|| bad("data.region required"))?;
        let active = shipping_rates::ActiveModel {
            region: Set(region),
            base_fee: Set(data.base_fee.unwrap_or(0)),
            per_unit_fee: Set(Some(data.per_unit_fee.unwrap_or(0))),
            currency: Set(data.currency.unwrap_or_else(|| "CNY".to_string())),
            status: Set(data
                .status
                .map(|v| if v == 1 { "ACTIVE" } else { "INACTIVE" }.to_string())
                .unwrap_or_else(|| "ACTIVE".to_string())),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<shippingv1::UpdateShippingRateRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = shipping_rates::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("shipping rate not found"))?;
        let mut active: shipping_rates::ActiveModel = existing.into();
        if let Some(v) = data.region {
            active.region = Set(v);
        }
        if let Some(v) = data.base_fee {
            active.base_fee = Set(v);
        }
        if let Some(v) = data.per_unit_fee {
            active.per_unit_fee = Set(Some(v));
        }
        if let Some(v) = data.status {
            active.status = Set(if v == 1 { "ACTIVE" } else { "INACTIVE" }.to_string());
        }
        active.updated_at = Set(Some(store::now()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<shippingv1::DeleteShippingRateRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(shippingv1::delete_shipping_rate_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        shipping_rates::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

pub struct ShipmentService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl shippingv1::shipment_service_server::ShipmentService for ShipmentService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<shippingv1::ListShipmentResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            shipments::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(shippingv1::ListShipmentResponse {
            items: rows.into_iter().map(shipment_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<shippingv1::CountShipmentResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            shipments::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(shippingv1::CountShipmentResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<shippingv1::GetShipmentRequest>,
    ) -> Result<Response<shippingv1::Shipment>, Status> {
        let req = request.into_inner();
        let Some(shippingv1::get_shipment_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = shipments::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("shipment not found"))?;
        Ok(Response::new(shipment_proto(row)))
    }

    async fn create(
        &self,
        request: Request<shippingv1::CreateShipmentRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let order_id = data.order_id.ok_or_else(|| bad("data.orderId required"))?;
        let active = shipments::ActiveModel {
            order_id: Set(order_id),
            user_id: Set(data.user_id),
            carrier: Set(data.carrier),
            tracking_number: Set(data.tracking_number),
            status: Set(data
                .status
                .map(|v| {
                    match shippingv1::shipment::Status::try_from(v).ok() {
                        Some(shippingv1::shipment::Status::Shipped) => "SHIPPED",
                        Some(shippingv1::shipment::Status::Delivered) => "DELIVERED",
                        _ => "PENDING",
                    }
                    .to_string()
                })
                .unwrap_or_else(|| "PENDING".to_string())),
            tracking_events: Set(data.tracking_events),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<shippingv1::UpdateShipmentRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = shipments::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("shipment not found"))?;
        let mut active: shipments::ActiveModel = existing.into();
        if let Some(v) = data.carrier {
            active.carrier = Set(Some(v));
        }
        if let Some(v) = data.tracking_number {
            active.tracking_number = Set(Some(v));
        }
        if let Some(v) = data.tracking_events {
            active.tracking_events = Set(Some(v));
        }
        if let Some(v) = data.status {
            active.status = Set(match shippingv1::shipment::Status::try_from(v).ok() {
                Some(shippingv1::shipment::Status::Shipped) => "SHIPPED",
                Some(shippingv1::shipment::Status::Delivered) => "DELIVERED",
                _ => "PENDING",
            }
            .to_string());
        }
        active.updated_at = Set(Some(store::now()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<shippingv1::DeleteShipmentRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(shippingv1::delete_shipment_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        shipments::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}
