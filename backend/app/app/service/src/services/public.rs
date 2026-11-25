//! The app face's hand behaviors — the storefront's payment intent
//! mapping (the app response is the flat shape the pay dialog reads:
//! transaction id + pay type + amount, with the pay url/qr slots for
//! the redirect/QR channels) and the mock confirm pass-through.

use crate::state::StatusError;
use tonic::Status;

use crate::state::AppState;
use shop_proto::proto::catalog::service::v1 as catalog_v1;
use shop_proto::proto::payment::service::v1 as payment_v1;

/// The acquiring-channel enum → the dialog's pay-type enum
/// (mock/QR/redirect; unknown channels degrade to mock so the demo
/// confirm path stays reachable).
fn pay_type_of(channel: Option<i32>) -> i32 {
    match payment_v1::PaymentChannelType::try_from(channel.unwrap_or(0)).ok() {
        Some(payment_v1::PaymentChannelType::PaymentChannelWechatNative) => {
            payment_v1::PayType::Qr as i32
        }
        Some(payment_v1::PaymentChannelType::PaymentChannelWechatH5) => {
            payment_v1::PayType::Redirect as i32
        }
        Some(payment_v1::PaymentChannelType::PaymentChannelAlipayPage) => {
            payment_v1::PayType::Redirect as i32
        }
        _ => payment_v1::PayType::Mock as i32,
    }
}

/// The payment intent: forward to the core's intent face, then flatten
/// the domain response into the app dialog's shape.
pub async fn create_payment_intent(
    state: &std::sync::Arc<AppState>,
    _ctx: &rushwind_http_binding::ctx::RequestContext,
    msg: payment_v1::CreatePaymentIntentRequest,
) -> Result<payment_v1::CreatePaymentIntentAppResponse, StatusError> {
    use shop_proto::proto::payment::service::v1::payment_transaction_service_client::PaymentTransactionServiceClient;
    let mut core = PaymentTransactionServiceClient::new(state.core_channel.clone());
    let resp = core
        .create_payment_intent(tonic::Request::new(msg))
        .await
        .map_err(crate::services::map_status)?
        .into_inner();
    let tx = resp.transaction;
    let channel = resp.channel;
    Ok(payment_v1::CreatePaymentIntentAppResponse {
        transaction_id: tx.as_ref().and_then(|t| t.id).unwrap_or(0),
        status: tx.as_ref().and_then(|t| t.status).unwrap_or(0),
        pay_type: pay_type_of(channel.as_ref().and_then(|c| c.channel)),
        pay_url: String::new(),
        qr_code: String::new(),
        amount: tx.as_ref().and_then(|t| t.amount).unwrap_or(0),
        currency: tx
            .as_ref()
            .and_then(|t| t.currency.clone())
            .unwrap_or_default(),
    })
}

/// The mock confirm: a synthetic gateway trade no rides the core's
/// confirm face (PENDING → SUCCEEDED → the order walks to PAID).
pub async fn mock_confirm_payment(
    state: &std::sync::Arc<AppState>,
    _ctx: &rushwind_http_binding::ctx::RequestContext,
    msg: payment_v1::MockConfirmPaymentRequest,
) -> Result<payment_v1::MockConfirmPaymentResponse, StatusError> {
    use shop_proto::proto::payment::service::v1::payment_transaction_service_client::PaymentTransactionServiceClient;
    let mut core = PaymentTransactionServiceClient::new(state.core_channel.clone());
    let resp = core
        .confirm_payment(tonic::Request::new(payment_v1::ConfirmPaymentRequest {
            transaction_id: msg.transaction_id,
            gateway_trade_no: format!(
                "MOCK-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            ),
            ..Default::default()
        }))
        .await
        .map_err(crate::services::map_status)?
        .into_inner();
    Ok(payment_v1::MockConfirmPaymentResponse {
        status: resp.status,
    })
}

/// The member-tier storefront faces: "my tier" (the login state decides
/// whose ladder — the client never names a user) and the anonymous
/// active-tier list.
pub async fn get_my_tier_info(
    state: &std::sync::Arc<AppState>,
    ctx: &rushwind_http_binding::ctx::RequestContext,
    mut msg: catalog_v1::GetUserTierInfoRequest,
) -> Result<catalog_v1::GetUserTierInfoResponse, StatusError> {
    // The login state is the only source of user_id — the query string
    // cannot name an arbitrary user.
    msg.user_id = ctx
        .claims
        .as_ref()
        .and_then(|c| c.get("uid"))
        .and_then(|v| v.as_u64())
        .map(|v| v as i64)
        .ok_or_else(|| StatusError::new(401, "UNAUTHORIZED", "user identity required"))?;
    use catalog_v1::member_tier_service_client::MemberTierServiceClient;
    let mut core = MemberTierServiceClient::new(state.core_channel.clone());
    let resp = core
        .get_user_tier_info(tonic::Request::new(msg))
        .await
        .map_err(crate::services::map_status)?
        .into_inner();
    Ok(resp)
}

/// The anonymous ladder: every ACTIVE tier, smallest level first (the
/// membership page renders the cards from it).
pub async fn list_tiers(
    state: &std::sync::Arc<AppState>,
    _ctx: &rushwind_http_binding::ctx::RequestContext,
    _msg: catalog_v1::ListActiveMemberTiersRequest,
) -> Result<catalog_v1::ListMemberTierResponse, StatusError> {
    use shop_proto::proto::pagination::PagingRequest;
    let mut core = catalog_v1::member_tier_service_client::MemberTierServiceClient::new(
        state.core_channel.clone(),
    );
    let resp = core
        .list(tonic::Request::new(PagingRequest {
            page: Some(1),
            page_size: Some(100),
            no_paging: Some(true),
            ..Default::default()
        }))
        .await
        .map_err(crate::services::map_status)?
        .into_inner();
    let items: Vec<catalog_v1::MemberTier> = resp
        .items
        .into_iter()
        .filter(|t| t.status == Some(catalog_v1::member_tier::Status::Active as i32))
        .collect();
    let total = items.len() as i64;
    Ok(catalog_v1::ListMemberTierResponse { items, total })
}
