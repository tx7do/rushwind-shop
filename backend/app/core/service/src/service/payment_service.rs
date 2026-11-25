//! The payment services — the transaction ledger, the refunds, the
//! channel configs, and the intent/confirm faces (the reference's
//! payment_service.go). ConfirmPayment is the bridge into the trade
//! state machine: the transaction's PENDING → SUCCEEDED (first
//! success only — the idempotent replay answers first_success=false)
//! walks the owning order PENDING_PAYMENT → PAID.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{orders, payment_channel_configs, payment_refunds, payment_transactions};
use store::paging::fetch_paged;

use shop_proto::proto::pagination::PagingRequest;
use shop_proto::proto::payment::service::v1 as paymentv1;

fn status_to_i32(stored: &str) -> i32 {
    use paymentv1::payment_transaction::Status;
    (match stored {
        "SUCCEEDED" => Status::Succeeded,
        "FAILED" => Status::Failed,
        "REFUNDED" => Status::Refunded,
        _ => Status::Pending,
    }) as i32
}

fn tx_proto(r: payment_transactions::Model) -> paymentv1::PaymentTransaction {
    paymentv1::PaymentTransaction {
        id: Some(r.id),
        order_id: Some(r.order_id),
        user_id: r.user_id,
        amount: Some(r.amount),
        status: Some(status_to_i32(&r.status)),
        currency: Some(r.currency.clone()),
        business_ref_id: r.business_ref_id.clone(),
        idempotency_key: r.idempotency_key.clone(),
        payment_method: r
            .payment_method
            .as_deref()
            .and_then(|v| payment_method_to_i32(v)),
        business_type: r
            .business_type
            .as_deref()
            .and_then(|v| business_type_to_i32(v)),
        gateway_trade_no: r.gateway_trade_no.clone(),
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        deleted_at: r.deleted_at.and_then(ts_to_proto),
        created_by: r.created_by,
        updated_by: r.updated_by,
        deleted_by: r.deleted_by,
    }
}

fn refund_proto(r: payment_refunds::Model) -> paymentv1::PaymentRefund {
    paymentv1::PaymentRefund {
        id: Some(r.id),
        transaction_id: Some(r.transaction_id),
        amount: Some(r.amount),
        status: Some(status_to_i32(&r.status)),
        currency: Some(r.currency.clone()),
        business_ref_id: r.business_ref_id.clone(),
        idempotency_key: r.idempotency_key.clone(),
        user_id: r.user_id,
        ..Default::default()
    }
}

fn channel_proto(r: payment_channel_configs::Model) -> paymentv1::PaymentChannelConfig {
    paymentv1::PaymentChannelConfig {
        id: Some(r.id),
        name: Some(r.name.clone()),
        channel: Some(
            channel_to_i32(r.channel.as_str())
                .unwrap_or(paymentv1::PaymentChannelType::PaymentChannelUnspecified as i32),
        ),
        app_id: r.app_id.clone(),
        mch_id: r.mch_id.clone(),
        sandbox: r.sandbox,
        enabled: r.enabled,
        credentials_configured: Some(r.credentials.is_some()),
        remark: r.remark.clone(),
        credentials: None,
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        created_by: r.created_by,
        updated_by: r.updated_by,
    }
}

/// The transaction's guarded status move (the ledger's only write path
/// for status).
async fn transition_tx(
    state: &AppState,
    id: i64,
    from: &[&str],
    to: &str,
    gateway_trade_no: Option<String>,
) -> Result<payment_transactions::Model, Status> {
    let existing = payment_transactions::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .map_err(db_status)?
        .ok_or_else(|| Status::not_found("payment transaction not found"))?;
    if !from.contains(&existing.status.as_str()) {
        return Err(Status::failed_precondition(format!(
            "payment transaction {} is {}, the transition to {} is not allowed",
            id, existing.status, to
        )));
    }
    let mut active: payment_transactions::ActiveModel = existing.into();
    active.status = Set(to.to_string());
    if let Some(no) = gateway_trade_no {
        active.gateway_trade_no = Set(Some(no));
    }
    active.updated_at = Set(Some(store::now().into()));
    let row = active.update(&state.db).await.map_err(db_status)?;
    Ok(row)
}

/// The payment-ledger → trade-machine bridge: the owning order walks
/// PENDING_PAYMENT → PAID on the first successful confirm.
async fn mark_order_paid(state: &AppState, order_id: i64) -> Result<(), Status> {
    let existing = orders::Entity::find_by_id(order_id)
        .one(&state.db)
        .await
        .map_err(db_status)?
        .ok_or_else(|| Status::not_found("order not found"))?;
    if existing.status != "PENDING_PAYMENT" {
        return Ok(()); // already walked — the idempotent replay
    }
    let mut active: orders::ActiveModel = existing.into();
    active.status = Set("PAID".to_string());
    active.updated_at = Set(Some(store::now().into()));
    active.update(&state.db).await.map_err(db_status)?;
    Ok(())
}

pub struct PaymentTransactionService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl paymentv1::payment_transaction_service_server::PaymentTransactionService
    for PaymentTransactionService
{
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<paymentv1::ListPaymentTransactionResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            payment_transactions::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(paymentv1::ListPaymentTransactionResponse {
            items: rows.into_iter().map(tx_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<paymentv1::CountPaymentTransactionResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            payment_transactions::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(paymentv1::CountPaymentTransactionResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<paymentv1::GetPaymentTransactionRequest>,
    ) -> Result<Response<paymentv1::PaymentTransaction>, Status> {
        let req = request.into_inner();
        let Some(paymentv1::get_payment_transaction_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = payment_transactions::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("payment transaction not found"))?;
        Ok(Response::new(tx_proto(row)))
    }

    async fn create(
        &self,
        request: Request<paymentv1::CreatePaymentTransactionRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let order_id = data.order_id.ok_or_else(|| bad("data.orderId required"))?;
        let active = payment_transactions::ActiveModel {
            order_id: Set(order_id),
            user_id: Set(data.user_id),
            amount: Set(data.amount.unwrap_or(0)),
            currency: Set(data.currency.clone().unwrap_or_else(|| "CNY".to_string())),
            status: Set(data
                .status
                .and_then(status_to_str_from_i32)
                .unwrap_or_else(|| "PENDING".to_string())),
            business_ref_id: Set(data.business_ref_id.clone()),
            idempotency_key: Set(data.idempotency_key.clone()),
            payment_method: Set(data.payment_method.and_then(payment_method_to_str)),
            business_type: Set(data.business_type.and_then(business_type_to_str)),
            gateway_trade_no: Set(data.gateway_trade_no.clone()),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<paymentv1::UpdatePaymentTransactionRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = payment_transactions::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("payment transaction not found"))?;
        let mut active: payment_transactions::ActiveModel = existing.into();
        if let Some(v) = &data.gateway_trade_no {
            active.gateway_trade_no = Set(Some(v.clone()));
        }
        if let Some(v) = data.status {
            active.status = Set(status_to_str_from_i32(v).unwrap_or_else(|| {
                active
                    .status
                    .clone()
                    .take()
                    .unwrap_or_else(|| "PENDING".into())
            }));
        }
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<paymentv1::DeletePaymentTransactionRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(paymentv1::delete_payment_transaction_request::QueryBy::Id(id)) = req.query_by
        else {
            return Err(bad("query_by required"));
        };
        payment_transactions::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The intent face: a PENDING transaction for the order (the mock
    /// channel answers with its own trade no — the real channels plug
    /// their SDKs here).
    async fn create_payment_intent(
        &self,
        request: Request<paymentv1::CreatePaymentIntentRequest>,
    ) -> Result<Response<paymentv1::CreatePaymentIntentResponse>, Status> {
        let req = request.into_inner();
        let channel = payment_channel_configs::Entity::find_by_id(req.channel_id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("channel config not found"))?;
        if channel.enabled != Some(true) {
            return Err(bad("channel disabled"));
        }
        let order = orders::Entity::find_by_id(req.order_id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("order not found"))?;
        let active = payment_transactions::ActiveModel {
            order_id: Set(req.order_id),
            user_id: Set(Some(req.user_id)),
            amount: Set(order.total_amount),
            currency: Set(order.currency.clone()),
            status: Set("PENDING".to_string()),
            idempotency_key: Set(Some(req.idempotency_key)),
            payment_method: Set(Some(channel.channel.clone())),
            business_type: Set(Some("ORDER".to_string())),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        let row = active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(paymentv1::CreatePaymentIntentResponse {
            transaction: Some(tx_proto(row)),
            channel: Some(channel_proto(channel)),
            subject: format!("Order #{} — RushWind Shop", req.order_id),
        }))
    }

    /// The confirm face: PENDING → SUCCEEDED (first success only),
    /// then the owning order walks PENDING_PAYMENT → PAID.
    async fn confirm_payment(
        &self,
        request: Request<paymentv1::ConfirmPaymentRequest>,
    ) -> Result<Response<paymentv1::ConfirmPaymentResponse>, Status> {
        let req = request.into_inner();
        let existing = payment_transactions::Entity::find_by_id(req.transaction_id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("payment transaction not found"))?;
        let first = existing.status == "PENDING";
        let row = transition_tx(
            &self.state,
            req.transaction_id,
            &["PENDING"],
            "SUCCEEDED",
            Some(req.gateway_trade_no.clone()),
        )
        .await?;
        let order_id = row.order_id;
        mark_order_paid(&self.state, order_id).await?;
        Ok(Response::new(paymentv1::ConfirmPaymentResponse {
            transaction_id: row.id,
            order_id,
            first_success: first,
            status: status_to_i32(&row.status),
        }))
    }
}

fn status_to_str_from_i32(v: i32) -> Option<String> {
    use paymentv1::payment_transaction::Status;
    Some(
        match Status::try_from(v).ok()? {
            Status::Pending => "PENDING",
            Status::Succeeded => "SUCCEEDED",
            Status::Failed => "FAILED",
            Status::Refunded => "REFUNDED",
            _ => return None,
        }
        .to_string(),
    )
}

/// The enum ↔ stored-spelling bridges (the demo data pins the bare
/// member names).
fn payment_method_to_str(v: i32) -> Option<String> {
    use paymentv1::PaymentMethod;
    Some(
        match PaymentMethod::try_from(v).ok()? {
            PaymentMethod::Balance => "BALANCE".to_string(),
            PaymentMethod::Credit => "CREDIT".to_string(),
            PaymentMethod::Wechat => "WECHAT".to_string(),
            PaymentMethod::Mock => "MOCK".to_string(),
            _ => return None,
        }
        .to_string(),
    )
}

#[allow(dead_code)]
fn payment_method_to_i32(stored: &str) -> Option<i32> {
    use paymentv1::PaymentMethod;
    Some(match stored {
        "BALANCE" => PaymentMethod::Balance,
        "CREDIT" => PaymentMethod::Credit,
        "WECHAT" => PaymentMethod::Wechat,
        "MOCK" => PaymentMethod::Mock,
        _ => return None,
    } as i32)
}

#[allow(dead_code)]
fn business_type_to_str(v: i32) -> Option<String> {
    use paymentv1::BusinessType;
    Some(match BusinessType::try_from(v).ok()? {
        BusinessType::Recharge => "RECHARGE".to_string(),
        BusinessType::Consume => "CONSUME".to_string(),
        BusinessType::Refund => "REFUND".to_string(),
        BusinessType::Gift => "GIFT".to_string(),
        _ => "CONSUME".to_string(),
    })
}

#[allow(dead_code)]
fn business_type_to_i32(stored: &str) -> Option<i32> {
    use paymentv1::BusinessType;
    Some(match stored {
        "RECHARGE" => BusinessType::Recharge as i32,
        "REFUND" => BusinessType::Refund as i32,
        "GIFT" => BusinessType::Gift as i32,
        _ => BusinessType::Consume as i32,
    })
}

#[allow(dead_code)]
fn channel_to_i32(stored: &str) -> Option<i32> {
    use paymentv1::PaymentChannelType;
    Some(match stored {
        "PAYMENT_CHANNEL_MOCK" => PaymentChannelType::PaymentChannelMock,
        "PAYMENT_CHANNEL_WECHAT_NATIVE" => PaymentChannelType::PaymentChannelWechatNative,
        "PAYMENT_CHANNEL_WECHAT_H5" => PaymentChannelType::PaymentChannelWechatH5,
        "PAYMENT_CHANNEL_ALIPAY_PAGE" => PaymentChannelType::PaymentChannelAlipayPage,
        _ => return None,
    } as i32)
}

pub struct PaymentRefundService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl paymentv1::payment_refund_service_server::PaymentRefundService for PaymentRefundService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<paymentv1::ListPaymentRefundResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            payment_refunds::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(paymentv1::ListPaymentRefundResponse {
            items: rows.into_iter().map(refund_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<paymentv1::CountPaymentRefundResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            payment_refunds::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(paymentv1::CountPaymentRefundResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<paymentv1::GetPaymentRefundRequest>,
    ) -> Result<Response<paymentv1::PaymentRefund>, Status> {
        let req = request.into_inner();
        let Some(paymentv1::get_payment_refund_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = payment_refunds::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("refund not found"))?;
        Ok(Response::new(refund_proto(row)))
    }

    async fn create(
        &self,
        request: Request<paymentv1::CreatePaymentRefundRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let transaction_id = data
            .transaction_id
            .ok_or_else(|| bad("data.transactionId required"))?;
        let active = payment_refunds::ActiveModel {
            transaction_id: Set(transaction_id),
            amount: Set(data.amount.unwrap_or(0)),
            status: Set(data
                .status
                .and_then(status_to_str_from_i32)
                .unwrap_or_else(|| "PENDING".to_string())),
            currency: Set(data.currency.unwrap_or_else(|| "CNY".to_string())),
            business_ref_id: Set(data.business_ref_id.clone()),
            idempotency_key: Set(data.idempotency_key.clone()),
            user_id: Set(data.user_id),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<paymentv1::UpdatePaymentRefundRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = payment_refunds::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("refund not found"))?;
        let mut active: payment_refunds::ActiveModel = existing.into();
        if let Some(v) = data.status {
            active.status = Set(status_to_str_from_i32(v).unwrap_or_else(|| "PENDING".to_string()));
        }
        if let Some(v) = data.amount {
            active.amount = Set(v);
        }
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<paymentv1::DeletePaymentRefundRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(paymentv1::delete_payment_refund_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        payment_refunds::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

pub struct PaymentChannelConfigService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl paymentv1::payment_channel_config_service_server::PaymentChannelConfigService
    for PaymentChannelConfigService
{
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<paymentv1::ListPaymentChannelConfigResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            payment_channel_configs::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(paymentv1::ListPaymentChannelConfigResponse {
            items: rows.into_iter().map(channel_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<paymentv1::CountPaymentChannelConfigResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            payment_channel_configs::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(
            paymentv1::CountPaymentChannelConfigResponse {
                count: total as i64,
            },
        ))
    }

    async fn get(
        &self,
        request: Request<paymentv1::GetPaymentChannelConfigRequest>,
    ) -> Result<Response<paymentv1::PaymentChannelConfig>, Status> {
        let req = request.into_inner();
        let row = payment_channel_configs::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("channel config not found"))?;
        Ok(Response::new(channel_proto(row)))
    }
}
