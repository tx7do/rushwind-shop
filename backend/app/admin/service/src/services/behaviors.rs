//! The admin face's hand behaviors — each hand method lands here as
//! its domain reaches this deployment (see the cms reference for the
//! shape: the operator stamp, the multipart bridges, the not-yet-built
//! contact faces).

use crate::state::StatusError;
use tonic::Status;

use crate::state::AppState;

/// The captcha pair: a straight forward to the core's
/// GenerateCaptcha — the core mints the 4-digit code, stores it
/// against the Redis session store (5-minute TTL) and renders the SVG
/// the login page shows. The login's own verify stays the core's.
pub async fn generate_captcha(
    state: &std::sync::Arc<AppState>,
    _ctx: &rushwind_http_binding::ctx::RequestContext,
    _msg: pbjson_types::Empty,
) -> Result<shop_proto::proto::authentication::service::v1::GenerateCaptchaResponse, StatusError> {
    use shop_proto::proto::authentication::service::v1::authentication_service_client::AuthenticationServiceClient;
    let mut core = AuthenticationServiceClient::new(state.core_channel.clone());
    Ok(core
        .generate_captcha(tonic::Request::new(pbjson_types::Empty {}))
        .await
        .map_err(crate::services::map_status)?
        .into_inner())
}
