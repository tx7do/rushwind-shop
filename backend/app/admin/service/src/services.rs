//! The service implementations, one module per BFF service. Modules
//! absent from this list ride the generator's null placeholders. The
//! proxies module is generated (scripts/gen-proxies.py); the hand faces
//! (authentication — captcha/cookie concerns; admin-portal aggregation)
//! land with their domains.

pub mod admin_portal;
pub mod behaviors;
pub mod proxies;
pub mod proxy_kit;

use crate::state::StatusError;

/// The tonic Status → the admin BFF's four-field envelope (the
/// error-table anchors the reason when the code maps onto one).
pub fn map_status(e: tonic::Status) -> StatusError {
    // The core's reason-carrying envelope rides the status details —
    // recover the resolved (status, reason, message) verbatim when the
    // reason resolves against the generated error tables (the recovered
    // string must anchor onto a 'static literal from that table).
    if let Some((status, reason, message)) = recover_envelope(&e) {
        return StatusError::new(status, reason, message);
    }
    let reason: &'static str = match e.code() {
        tonic::Code::InvalidArgument => "BAD_REQUEST",
        tonic::Code::Unauthenticated => "UNAUTHORIZED",
        tonic::Code::PermissionDenied => "FORBIDDEN",
        tonic::Code::NotFound => "NOT_FOUND",
        tonic::Code::AlreadyExists | tonic::Code::Aborted => "CONFLICT",
        _ => "",
    };
    if reason.is_empty() {
        crate::state::internal_error(e.message())
    } else {
        crate::state::status_error(reason, e.message())
    }
}

/// Recovers the core's envelope off the status details. The JSON payload
/// is `{status, reason, message}` — but only a reason that anchors onto
/// a generated error-table literal is trusted; anything else yields
/// `None` and the caller falls back to the coarse code mapping.
fn recover_envelope(e: &tonic::Status) -> Option<(i32, &'static str, String)> {
    let v: serde_json::Value = serde_json::from_slice(e.details()).ok()?;
    let reason = v.get("reason")?.as_str()?;
    let status = i32::try_from(v.get("status")?.as_i64()?).ok()?;
    let message = v
        .get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_string();
    // The 'static anchor: the reason must appear in the owning table's
    // literal set (the leak is bounded — the table is finite and
    // process-static).
    let anchored = shop_proto::gen_admin::error_tables::ERROR_STATUS
        .iter()
        .chain(shop_proto::gen_app::error_tables::ERROR_STATUS.iter())
        .find(|(_, r, _)| *r == reason)
        .map(|(_, r, _)| *r)?;
    Some((status, anchored, message))
}

/// The operator stamp: the verified claims ride as gRPC metadata —
/// the core service reads it where the proto carries no user field.
pub fn with_operator<T>(
    ctx: &rushwind_http_binding::ctx::RequestContext,
    msg: T,
) -> tonic::Request<T> {
    let mut req = tonic::Request::new(msg);
    if let Some(claims) = &ctx.claims {
        if let Some(uid) = claims.get("uid").and_then(|v| v.as_u64()) {
            if let Ok(v) = uid.to_string().parse() {
                req.metadata_mut().insert("x-user-id", v);
            }
        }
        if let Some(tid) = claims.get("tid").and_then(|v| v.as_u64()) {
            if let Ok(v) = tid.to_string().parse() {
                req.metadata_mut().insert("x-tenant-id", v);
            }
        }
    }
    req
}
