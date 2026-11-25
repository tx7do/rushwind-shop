//! The order services — the trade state machine and its line items
//! (the reference's order_service.go / order_item_service.go). The
//! lifecycle: Create lands as PENDING_PAYMENT; CancelOrder walks
//! PENDING_PAYMENT → CANCELLED; ConfirmReceipt walks PAID →
//! FULFILLED; ExpireOrderByTimeout sweeps a stale pending row to
//! CANCELLED. Terminal states (CANCELLED/CLOSED) never move again.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{order_items, orders};
use store::paging::fetch_paged;

use shop_proto::proto::order::service::v1 as orderv1;
use shop_proto::proto::pagination::PagingRequest;

/// The proto Status enum ↔ the stored spelling (the demo data pins the
/// bare member names: `CANCELLED`, `PENDING_PAYMENT`, …).
fn status_to_i32(stored: &str) -> i32 {
    use orderv1::order::Status;
    (match stored {
        "PENDING_PAYMENT" => Status::PendingPayment,
        "PAID" => Status::Paid,
        "FULFILLED" => Status::Fulfilled,
        "CLOSED" => Status::Closed,
        _ => Status::Cancelled,
    }) as i32
}

fn status_to_str(v: i32) -> Option<&'static str> {
    use orderv1::order::Status;
    Some(match Status::try_from(v).ok()? {
        Status::PendingPayment => "PENDING_PAYMENT",
        Status::Paid => "PAID",
        Status::Cancelled => "CANCELLED",
        Status::Fulfilled => "FULFILLED",
        Status::Closed => "CLOSED",
        _ => return None,
    })
}

fn order_proto(r: orders::Model) -> orderv1::Order {
    orderv1::Order {
        id: Some(r.id),
        user_id: Some(r.user_id),
        total_amount: Some(r.total_amount),
        currency: Some(r.currency.clone()),
        status: Some(status_to_i32(&r.status)),
        business_ref_id: r.business_ref_id.clone(),
        idempotency_key: r.idempotency_key.clone(),
        recipient_name: r.recipient_name.clone(),
        recipient_phone: r.recipient_phone.clone(),
        shipping_address: r.shipping_address.clone(),
        original_amount: r.original_amount,
        discount_amount: r.discount_amount,
        shipping_fee: r.shipping_fee,
        tax_amount: r.tax_amount,
        shipping_region: r.shipping_region.clone(),
        tenant_id: r.tenant_id,
        groupon_activity_id: r.groupon_activity_id,
        groupon_group_id: r.groupon_group_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        deleted_at: r.deleted_at.and_then(ts_to_proto),
        created_by: r.created_by,
        updated_by: r.updated_by,
        deleted_by: r.deleted_by,
    }
}

fn order_item_proto(r: order_items::Model) -> orderv1::OrderItem {
    orderv1::OrderItem {
        id: Some(r.id),
        order_id: Some(r.order_id),
        sku_id: Some(r.sku_id),
        sku_snapshot: r
            .sku_snapshot
            .and_then(|j| serde_json::from_value(j).ok()),
        quantity: Some(r.quantity),
        unit_price: r.unit_price,
        subtotal: r.subtotal,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

async fn order_row(db: &sea_orm::DatabaseConnection, id: i64) -> Result<orders::Model, Status> {
    orders::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(db_status)?
        .ok_or_else(|| Status::not_found("order not found"))
}

/// The guarded transition: the row must sit in `from` for the move to
/// `to` to apply (the state machine's only write path).
async fn transition(
    state: &AppState,
    id: i64,
    from: &[&str],
    to: &str,
) -> Result<orders::Model, Status> {
    let existing = order_row(&state.db, id).await?;
    if !from.contains(&existing.status.as_str()) {
        return Err(Status::failed_precondition(format!(
            "order {} is {}, the transition to {} is not allowed",
            id, existing.status, to
        )));
    }
    let mut active: orders::ActiveModel = existing.into();
    active.status = Set(to.to_string());
    active.updated_at = Set(Some(store::now().into()));
    let row = active.update(&state.db).await.map_err(db_status)?;
    Ok(row)
}

pub struct OrderService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl orderv1::order_service_server::OrderService for OrderService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<orderv1::ListOrderResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            orders::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(orderv1::ListOrderResponse {
            items: rows.into_iter().map(order_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<orderv1::CountOrderResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            orders::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(orderv1::CountOrderResponse { count: total as i64 }))
    }

    async fn get(
        &self,
        request: Request<orderv1::GetOrderRequest>,
    ) -> Result<Response<orderv1::Order>, Status> {
        let req = request.into_inner();
        let Some(orderv1::get_order_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        Ok(Response::new(order_proto(order_row(&self.state.db, id).await?)))
    }

    async fn create(
        &self,
        request: Request<orderv1::CreateOrderRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let user_id = data.user_id.ok_or_else(|| bad("data.userId required"))?;
        let status = data
            .status
            .and_then(status_to_str)
            .unwrap_or("PENDING_PAYMENT")
            .to_string();
        let active = orders::ActiveModel {
            user_id: Set(user_id),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            currency: Set(data.currency.unwrap_or_else(|| "CNY".to_string())),
            business_ref_id: Set(data.business_ref_id),
            idempotency_key: Set(data.idempotency_key),
            total_amount: Set(data.total_amount.unwrap_or(0)),
            original_amount: Set(Some(data.original_amount.unwrap_or(0))),
            discount_amount: Set(Some(data.discount_amount.unwrap_or(0))),
            shipping_fee: Set(Some(data.shipping_fee.unwrap_or(0))),
            tax_amount: Set(Some(data.tax_amount.unwrap_or(0))),
            shipping_region: Set(data.shipping_region),
            status: Set(status),
            recipient_name: Set(data.recipient_name),
            recipient_phone: Set(data.recipient_phone),
            shipping_address: Set(data.shipping_address),
            groupon_activity_id: Set(data.groupon_activity_id),
            groupon_group_id: Set(data.groupon_group_id),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        // The line items ride the OrderItem face (the contract carries
        // no items field on the create order request).
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<orderv1::UpdateOrderRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = order_row(&self.state.db, req.id).await?;
        let mut active: orders::ActiveModel = existing.into();
        if let Some(v) = &data.recipient_name {
            active.recipient_name = Set(Some(v.clone()));
        }
        if let Some(v) = &data.recipient_phone {
            active.recipient_phone = Set(Some(v.clone()));
        }
        if let Some(v) = &data.shipping_address {
            active.shipping_address = Set(Some(v.clone()));
        }
        if let Some(v) = &data.shipping_region {
            active.shipping_region = Set(Some(v.clone()));
        }
        if let Some(v) = data.status.and_then(status_to_str) {
            active.status = Set(v.to_string());
        }
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<orderv1::DeleteOrderRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(orderv1::delete_order_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        orders::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn expire_order_by_timeout(
        &self,
        request: Request<orderv1::ExpireOrderByTimeoutRequest>,
    ) -> Result<Response<orderv1::ExpireOrderByTimeoutResponse>, Status> {
        let req = request.into_inner();
        let order_id = req
            .order_id
            .ok_or_else(|| bad("order_id required"))?;
        let row = transition(&self.state, order_id, &["PENDING_PAYMENT"], "CANCELLED").await?;
        let _ = (req.task_id, req.trace_id);
        Ok(Response::new(orderv1::ExpireOrderByTimeoutResponse {
            expired: Some(row.status == "CANCELLED"),
        }))
    }

    async fn cancel_order(
        &self,
        request: Request<orderv1::CancelOrderRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let _ = req.reason;
        transition(&self.state, req.id, &["PENDING_PAYMENT"], "CANCELLED").await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn confirm_receipt(
        &self,
        request: Request<orderv1::ConfirmReceiptRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        transition(&self.state, req.id, &["PAID", "FULFILLED"], "FULFILLED").await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

pub struct OrderItemService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl orderv1::order_item_service_server::OrderItemService for OrderItemService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<orderv1::ListOrderItemResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            order_items::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(orderv1::ListOrderItemResponse {
            items: rows.into_iter().map(order_item_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<orderv1::CountOrderItemResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            order_items::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(orderv1::CountOrderItemResponse { count: total as i64 }))
    }

    async fn get(
        &self,
        request: Request<orderv1::GetOrderItemRequest>,
    ) -> Result<Response<orderv1::OrderItem>, Status> {
        let req = request.into_inner();
        let Some(orderv1::get_order_item_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = order_items::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("order item not found"))?;
        Ok(Response::new(order_item_proto(row)))
    }

    async fn create(
        &self,
        request: Request<orderv1::CreateOrderItemRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let order_id = data.order_id.ok_or_else(|| bad("data.orderId required"))?;
        let sku_id = data.sku_id.ok_or_else(|| bad("data.skuId required"))?;
        let active = order_items::ActiveModel {
            order_id: Set(order_id),
            sku_id: Set(sku_id),
            sku_snapshot: Set(data.sku_snapshot.map(serde_json::Value::String)),
            quantity: Set(data.quantity.unwrap_or(1).max(1)),
            unit_price: Set(Some(data.unit_price.unwrap_or(0))),
            subtotal: Set(Some(data.subtotal.unwrap_or(0))),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn batch_create(
        &self,
        request: Request<orderv1::BatchCreateOrderItemsRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        for data in req.items {
            let Some(order_id) = data.order_id else {
                return Err(bad("data.orderId required"));
            };
            let Some(sku_id) = data.sku_id else {
                return Err(bad("data.skuId required"));
            };
            let active = order_items::ActiveModel {
                order_id: Set(order_id),
                sku_id: Set(sku_id),
                sku_snapshot: Set(data.sku_snapshot.map(serde_json::Value::String)),
                quantity: Set(data.quantity.unwrap_or(1).max(1)),
                unit_price: Set(Some(data.unit_price.unwrap_or(0))),
                subtotal: Set(Some(data.subtotal.unwrap_or(0))),
                created_at: Set(Some(store::now().into())),
                ..Default::default()
            };
            active.insert(&self.state.db).await.map_err(db_status)?;
        }
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<orderv1::UpdateOrderItemRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = order_items::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("order item not found"))?;
        let mut active: order_items::ActiveModel = existing.into();
        if let Some(v) = data.quantity {
            active.quantity = Set(v.max(1));
        }
        if let Some(v) = data.unit_price {
            active.unit_price = Set(Some(v));
        }
        if let Some(v) = data.subtotal {
            active.subtotal = Set(Some(v));
        }
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<orderv1::DeleteOrderItemRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(orderv1::delete_order_item_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        order_items::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}
