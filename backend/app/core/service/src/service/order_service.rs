//! The order services — the trade state machine and its line items
//! (the reference's order_service.go / order_item_service.go). The
//! lifecycle: Create lands as PENDING_PAYMENT; CancelOrder walks
//! PENDING_PAYMENT → CANCELLED; ConfirmReceipt walks PAID →
//! FULFILLED; ExpireOrderByTimeout sweeps a stale pending row to
//! CANCELLED. Terminal states (CANCELLED/CLOSED) never move again.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{cart_items, carts, order_items, orders, sku_prices, skus, stock_op_logs};
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
        sku_snapshot: r.sku_snapshot.and_then(|j| serde_json::from_value(j).ok()),
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
    active.updated_at = Set(Some(store::now()));
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
        Ok(Response::new(orderv1::CountOrderResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<orderv1::GetOrderRequest>,
    ) -> Result<Response<orderv1::Order>, Status> {
        let req = request.into_inner();
        let Some(orderv1::get_order_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        Ok(Response::new(order_proto(
            order_row(&self.state.db, id).await?,
        )))
    }

    async fn create(
        &self,
        request: Request<orderv1::CreateOrderRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let user_id = data.user_id.ok_or_else(|| bad("data.userId required"))?;
        if data
            .idempotency_key
            .as_ref()
            .map(|s| s.is_empty())
            .unwrap_or(true)
        {
            return Err(bad("idempotency_key is required"));
        }
        let tenant_id = data.tenant_id.unwrap_or(0);
        let currency = data.currency.unwrap_or_else(|| "CNY".to_string());

        // The storefront sends the ORDER SHELL only (totalAmount 0, no
        // items) — the conversion from the user's cart is the server's
        // job (the reference's tx flow): find the cart, price each item
        // off sku_prices, snapshot + insert order items, deduct stock
        // with a ledger row each, clear the cart, and stamp the computed
        // total back onto the order — all inside one transaction.
        let txn = self.state.db.begin().await.map_err(db_status)?;

        let cart = carts::Entity::find()
            .filter(carts::Column::TenantId.eq(tenant_id))
            .filter(carts::Column::UserId.eq(user_id))
            .one(&txn)
            .await
            .map_err(db_status)?
            .ok_or_else(|| bad("cart is empty or not found"))?;
        let cart_items = cart_items::Entity::find()
            .filter(cart_items::Column::CartId.eq(cart.id))
            .all(&txn)
            .await
            .map_err(db_status)?;
        if cart_items.is_empty() {
            return Err(bad("cart is empty"));
        }

        let active = orders::ActiveModel {
            user_id: Set(user_id),
            tenant_id: Set(Some(tenant_id)),
            currency: Set(currency.clone()),
            business_ref_id: Set(data.business_ref_id),
            idempotency_key: Set(data.idempotency_key),
            total_amount: Set(0), // recomputed below from the priced items
            original_amount: Set(Some(0)),
            discount_amount: Set(Some(0)),
            shipping_fee: Set(Some(0)),
            tax_amount: Set(Some(0)),
            shipping_region: Set(data.shipping_region),
            status: Set("PENDING_PAYMENT".to_string()), // the create-time state, client value ignored
            recipient_name: Set(data.recipient_name),
            recipient_phone: Set(data.recipient_phone),
            shipping_address: Set(data.shipping_address),
            groupon_activity_id: Set(data.groupon_activity_id),
            groupon_group_id: Set(data.groupon_group_id),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        let order = active.insert(&txn).await.map_err(db_status)?;
        let order_id = order.id;

        let mut total_amount: i64 = 0;
        for item in &cart_items {
            let qty = i64::from(item.quantity.max(1));
            let sku = skus::Entity::find_by_id(item.sku_id)
                .one(&txn)
                .await
                .map_err(db_status)?
                .ok_or_else(|| bad("sku not found"))?;
            if sku.stock_qty < item.quantity.max(1) {
                return Err(bad("insufficient stock for sku"));
            }
            let price = sku_prices::Entity::find()
                .filter(sku_prices::Column::SkuId.eq(item.sku_id))
                .filter(sku_prices::Column::Currency.eq(&currency))
                .one(&txn)
                .await
                .map_err(db_status)?
                .ok_or_else(|| bad("sku has no price for settlement currency"))?;
            let unit_price: i64 = price
                .amount
                .parse()
                .map_err(|_| bad("sku has invalid price"))?;
            if unit_price <= 0 {
                return Err(bad("sku has invalid price"));
            }
            let subtotal = unit_price * qty;
            total_amount += subtotal;

            let snapshot = serde_json::json!({
                "sku_id": item.sku_id,
                "quantity": item.quantity.max(1),
                "currency": currency,
                "unit_price": unit_price,
            });
            order_items::ActiveModel {
                order_id: Set(order_id),
                sku_id: Set(item.sku_id),
                sku_snapshot: Set(Some(snapshot)),
                quantity: Set(item.quantity.max(1)),
                unit_price: Set(Some(unit_price)),
                subtotal: Set(Some(subtotal)),
                created_at: Set(Some(store::now())),
                ..Default::default()
            }
            .insert(&txn)
            .await
            .map_err(db_status)?;

            let new_stock = sku.stock_qty - item.quantity.max(1);
            skus::ActiveModel {
                id: sea_orm::Unchanged(item.sku_id),
                stock_qty: Set(new_stock),
                ..Default::default()
            }
            .update(&txn)
            .await
            .map_err(db_status)?;
            stock_op_logs::ActiveModel {
                sku_id: Set(item.sku_id),
                product_id: Set(Some(sku.product_id)),
                delta: Set(-item.quantity.max(1)),
                stock_before: Set(Some(sku.stock_qty)),
                stock_after: Set(Some(new_stock)),
                reason: Set(Some("ORDER_DEDUCT".to_string())),
                ref_id: Set(Some(order_id)),
                created_at: Set(Some(store::now())),
                ..Default::default()
            }
            .insert(&txn)
            .await
            .map_err(db_status)?;
        }

        cart_items::Entity::delete_many()
            .filter(cart_items::Column::CartId.eq(cart.id))
            .exec(&txn)
            .await
            .map_err(db_status)?;

        // Stamp the server-computed total onto the order (the client's
        // totalAmount rides in as 0 by design).
        let mut stamped: orders::ActiveModel = order.into();
        stamped.total_amount = Set(total_amount);
        stamped.original_amount = Set(Some(total_amount));
        stamped.update(&txn).await.map_err(db_status)?;

        txn.commit().await.map_err(db_status)?;
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
        active.updated_at = Set(Some(store::now()));
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
        let order_id = req.order_id.ok_or_else(|| bad("order_id required"))?;
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
        Ok(Response::new(orderv1::CountOrderItemResponse {
            count: total as i64,
        }))
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
            created_at: Set(Some(store::now())),
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
                created_at: Set(Some(store::now())),
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
        active.updated_at = Set(Some(store::now()));
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
