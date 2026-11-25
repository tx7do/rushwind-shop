//! The online-session service — the Redis whitelist read out as the
//! session-management face (the reference's online_session_service.go):
//! the admin's List (per client/user filters), the self ListMy, the
//! ForceLogout kick, and the self RevokeMy.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::state::AppState;
use crate::token::ClientKind;

use shop_proto::proto::online_session::service::v1 as osv1;

/// The unverified-claims parser shared by the store's session read-out
/// (sub, tid, iat, exp off the JWT payload — the signature already
/// proved itself at mint time).
fn parse_unverified(token: &str) -> Option<(String, i64, i64, i64)> {
    use base64::Engine as _;
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()?;
    let v: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    let sub = v.get("sub")?.as_str()?.to_string();
    let tid = v.get("tid")?.as_i64()?;
    let iat = v.get("iat")?.as_i64()?;
    let exp = v.get("exp")?.as_i64()?;
    Some((sub, tid, iat, exp))
}

fn to_proto(
    s: crate::token::SessionInfo,
    current_jti: Option<&str>,
) -> osv1::OnlineSession {
    use chrono::TimeZone as _;
    osv1::OnlineSession {
        jti: Some(s.jti.clone()),
        user_id: Some(s.user_id),
        username: Some(s.username.clone()),
        tenant_id: Some(s.tenant_id),
        client_type: Some(if s.client == ClientKind::App {
            1
        } else {
            0
        }),
        login_at: chrono::Utc.timestamp_opt(s.login_at, 0).single().map(|dt| {
            pbjson_types::Timestamp {
                seconds: dt.timestamp(),
                nanos: 0,
            }
        }),
        current: current_jti.map(|c| c == s.jti),
        ..Default::default()
    }
}

fn kind_of(v: i32) -> ClientKind {
    ClientKind::from_i32(v)
}

pub struct OnlineSessionService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl osv1::online_session_service_server::OnlineSessionService for OnlineSessionService {
    /// The admin face: the live sessions of the requested client
    /// (client_type 0 = admin, 1 = app), optionally scoped to one user.
    async fn list(
        &self,
        request: Request<osv1::ListOnlineSessionRequest>,
    ) -> Result<Response<osv1::ListOnlineSessionResponse>, Status> {
        let req = request.into_inner();
        let mut rows = Vec::new();
        for client in [ClientKind::Admin, ClientKind::App] {
            rows.extend(
                self.state
                    .tokens(client)
                    .list_sessions(parse_unverified)
                    .await
                    .map_err(Status::internal)?,
            );
        }
        if let Some(kw) = &req.keyword {
            let kw = kw.to_lowercase();
            rows.retain(|s| {
                s.username.to_lowercase().contains(&kw) || s.jti.to_lowercase().contains(&kw)
            });
        }
        let page = req.page.unwrap_or(1).max(1) as usize;
        let page_size = (req.page_size.unwrap_or(20).max(1) as usize).min(100);
        let total = rows.len() as i64;
        let items = rows
            .into_iter()
            .skip((page - 1) * page_size)
            .take(page_size)
            .map(|s| to_proto(s, None))
            .collect();
        Ok(Response::new(osv1::ListOnlineSessionResponse {
            items,
            total,
        }))
    }

    /// The kick: revoke the session's both halves and blacklist the
    /// jti (the gate's session stage rejects it on the next request).
    async fn force_logout(
        &self,
        request: Request<osv1::ForceLogoutSessionRequest>,
    ) -> Result<Response<osv1::ForceLogoutSessionResponse>, Status> {
        let req = request.into_inner();
        let client = kind_of(req.client_type.unwrap_or(0));
        let jti = req.jti.clone().unwrap_or_default();
        let user_id = req.user_id.unwrap_or(0);
        // Locate the row's uid when the request left it out.
        let uid = if user_id != 0 {
            user_id
        } else {
            self.state
                .tokens(client)
                .list_sessions(parse_unverified)
                .await
                .map_err(Status::internal)?
                .into_iter()
                .find(|s| s.jti == jti)
                .map(|s| s.user_id)
                .ok_or_else(|| tonic::Status::not_found("session not found"))?
        };
        self.state
            .tokens(client)
            .revoke_session_global(uid, &jti)
            .await
            .map_err(Status::internal)?;
        Ok(Response::new(osv1::ForceLogoutSessionResponse::default()))
    }

    /// The self face: the caller's live sessions (the operator rides
    /// the forwarded x-user-id metadata; the token's own claims would
    /// join with the caller-claims bridge).
    async fn list_my(
        &self,
        request: Request<osv1::ListMyOnlineSessionRequest>,
    ) -> Result<Response<osv1::ListOnlineSessionResponse>, Status> {
        let req = request.into_inner();
        let uid = req.user_id.unwrap_or(0);
        if uid == 0 {
            return Err(tonic::Status::invalid_argument("user_id required"));
        }
        let mut rows = Vec::new();
        for client in [ClientKind::Admin, ClientKind::App] {
            rows.extend(
                self.state
                    .tokens(client)
                    .list_sessions(parse_unverified)
                    .await
                    .map_err(Status::internal)?,
            );
        }
        rows.retain(|s| s.user_id == uid);
        let total = rows.len() as i64;
        let items = rows
            .into_iter()
            .map(|s| to_proto(s, req.current_jti.as_deref()))
            .collect();
        Ok(Response::new(osv1::ListOnlineSessionResponse {
            items,
            total,
        }))
    }

    /// The self revocation: ownership checked against the live rows
    /// before the revoke.
    async fn revoke_my(
        &self,
        request: Request<osv1::RevokeMyOnlineSessionRequest>,
    ) -> Result<Response<osv1::RevokeMyOnlineSessionResponse>, Status> {
        let req = request.into_inner();
        let uid = req.user_id.unwrap_or(0);
        if uid == 0 {
            return Err(tonic::Status::invalid_argument("user_id required"));
        }
        let client = kind_of(req.client_type.unwrap_or(0));
        let jti = req.jti.clone().unwrap_or_default();
        let owns = self
            .state
            .tokens(client)
            .list_sessions(parse_unverified)
            .await
            .map_err(Status::internal)?
            .into_iter()
            .any(|s| s.user_id == uid && s.jti == jti);
        if !owns {
            return Err(tonic::Status::not_found("session not found"));
        }
        self.state
            .tokens(client)
            .revoke_session_global(uid, &jti)
            .await
            .map_err(Status::internal)?;
        Ok(Response::new(osv1::RevokeMyOnlineSessionResponse::default()))
    }
}
