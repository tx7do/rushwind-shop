//! The MFA service (the reference's mfa_service.go, core face) — this
//! round carries TOTP only. The management faces (status/list/start/
//! confirm/disable/revoke) demand the forwarded operator identity and
//! touch nobody else's factors; the login challenge face is the
//! password flow's second step: the challenge context was written by
//! the caller that checked the password, the verify path re-checks the
//! code, voids on the failure cap, adjudicates atomically, and issues
//! the real token pair. Backup codes and StartMFAChallenge stay
//! Unimplemented — the reference ships them the same way.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::data::mfa_challenge_cache::MfaChallengeCache;
use crate::data::mfa_factor_repo as repo;
use crate::service::context;
use crate::service::mfa_totp;
use crate::state::{bad, forbidden, ts_to_proto, AppState};

use shop_proto::proto::authentication::service::v1 as authv1;

fn enrolled_proto(i: repo::EnrolledFactorInfo) -> authv1::EnrolledMethod {
    authv1::EnrolledMethod {
        id: i.id.to_string(),
        method: authv1::MfaMethod::from_str_name(&i.method)
            .map(|e| e as i32)
            .unwrap_or(authv1::MfaMethod::Totp as i32),
        display: i.display_name,
        enabled: i.enabled,
        created_at: i.created_at.and_then(ts_to_proto),
        last_used_at: i.last_used_at.and_then(ts_to_proto),
    }
}

fn method_spelling(v: i32) -> Result<&'static str, Status> {
    match authv1::MfaMethod::try_from(v) {
        Ok(authv1::MfaMethod::Totp) => Ok("TOTP"),
        Ok(authv1::MfaMethod::Sms) => Ok("SMS"),
        Ok(authv1::MfaMethod::Email) => Ok("EMAIL"),
        Ok(authv1::MfaMethod::Webauthn) => Ok("WEBAUTHN"),
        Ok(authv1::MfaMethod::BackupCode) => Ok("BACKUP_CODE"),
        Ok(authv1::MfaMethod::U2f) => Ok("U2F"),
        _ => Err(bad("unsupported mfa method")),
    }
}

/// The challenge request's totp_code — the one supported verifier this
/// round.
fn challenge_totp_code(req: &authv1::VerifyMfaChallengeRequest) -> Option<&str> {
    match req.response.as_ref() {
        Some(authv1::verify_mfa_challenge_request::Response::TotpCode(code)) => Some(code.as_str()),
        _ => None,
    }
}

pub struct MfaService {
    pub state: Arc<AppState>,
}

impl MfaService {
    fn cache(&self) -> MfaChallengeCache {
        MfaChallengeCache::new(self.state.redis.clone())
    }

    /// The shared token-issuance shape (the authentication face's
    /// issue_pair, mirrored here so the challenge verify lands the real
    /// pair without round-tripping through the auth service).
    async fn issue_pair(
        &self,
        client: crate::token::ClientKind,
        user_id: i64,
        tenant_id: i64,
        username: &str,
    ) -> Result<(String, Option<String>), Status> {
        use rushwind_authn::Authenticator as _;
        let store = self.state.tokens(client);
        let jti = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let mut claims = serde_json::Map::new();
        claims.insert("sub".into(), username.into());
        claims.insert("uid".into(), user_id.into());
        claims.insert("tid".into(), tenant_id.into());
        claims.insert("iat".into(), now.into());
        claims.insert("exp".into(), (now + store.access_secs()).into());
        claims.insert("jti".into(), jti.clone().into());
        let access = self
            .state
            .jwt
            .create_identity(&rushwind_authn::AuthClaims(claims))
            .map_err(|_| Status::internal("create access token failed"))?;
        let refresh = (store.refresh_secs() > 0).then(crate::token::TokenStore::new_refresh_token);
        store
            .add_token_pair(user_id, &jti, &access, refresh.as_deref())
            .await
            .map_err(Status::internal)?;
        Ok((access, refresh))
    }

    fn login_response(
        &self,
        client: crate::token::ClientKind,
        access: String,
        refresh: Option<String>,
    ) -> authv1::LoginResponse {
        let store = self.state.tokens(client);
        authv1::LoginResponse {
            token_type: authv1::TokenType::Bearer as i32,
            access_token: access,
            refresh_token: refresh,
            expires_in: store.access_secs(),
            refresh_expires_in: Some(store.refresh_secs()),
            ..Default::default()
        }
    }

    /// The username the challenge payload carries (the verify response's
    /// token claims need it).
    fn payload_username(payload: &serde_json::Value) -> String {
        payload
            .get("sub")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    }

    fn payload_i64(payload: &serde_json::Value, key: &str) -> i64 {
        payload
            .get(key)
            .and_then(|v| v.as_i64())
            .or_else(|| {
                payload
                    .get(key)
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
            })
            .unwrap_or(0)
    }
}

#[async_trait::async_trait]
impl authv1::mfa_service_server::MfaService for MfaService {
    async fn get_mfa_status(
        &self,
        request: Request<authv1::GetMfaStatusRequest>,
    ) -> Result<Response<authv1::GetMfaStatusResponse>, Status> {
        let uid = context::operator_of(&request)?;
        let tid = context::request_tenant_of(&request);
        let infos = repo::list_by_user(&self.state.db, tid, uid).await?;
        let has_totp = infos.iter().any(|i| i.method == "TOTP" && i.enabled);
        let items: Vec<authv1::EnrolledMethod> = if has_totp {
            infos.into_iter().map(enrolled_proto).collect()
        } else {
            Vec::new()
        };
        Ok(Response::new(authv1::GetMfaStatusResponse {
            enabled: has_totp,
            enrolled: items,
            enforcement: (if has_totp {
                authv1::MfaEnforcement::MfaRequired
            } else {
                authv1::MfaEnforcement::MfaNotRequired
            }) as i32,
        }))
    }

    async fn list_enrolled_methods(
        &self,
        request: Request<authv1::ListEnrolledMethodsRequest>,
    ) -> Result<Response<authv1::ListEnrolledMethodsResponse>, Status> {
        let uid = context::operator_of(&request)?;
        let tid = context::request_tenant_of(&request);
        let infos = repo::list_by_user(&self.state.db, tid, uid).await?;
        Ok(Response::new(authv1::ListEnrolledMethodsResponse {
            items: infos.into_iter().map(enrolled_proto).collect(),
        }))
    }

    /// The enroll start: mint the secret, park it in the challenge
    /// cache, hand back the otpauth URL and the QR the app scans.
    async fn start_enroll_method(
        &self,
        request: Request<authv1::StartEnrollMethodRequest>,
    ) -> Result<Response<authv1::StartEnrollMethodResponse>, Status> {
        let uid = context::operator_of(&request)?;
        let tid = context::request_tenant_of(&request);
        let req = request.into_inner();
        if req.method != authv1::MfaMethod::Totp as i32 {
            return Err(bad("only TOTP is supported"));
        }
        let cache = self.cache();
        if !cache.try_acquire_enroll_cooldown(tid, uid).await {
            return Err(bad("enroll request too frequent, retry later"));
        }
        if repo::has_enabled_totp(&self.state.db, tid, uid).await? {
            return Err(bad("totp already enrolled, disable it first"));
        }
        let secret = mfa_totp::generate_totp_secret();
        let op_id = cache
            .set_enroll_challenge(&secret, tid, uid)
            .await
            .map_err(Status::internal)?;
        let url = mfa_totp::otpauth_url(&secret, uid);
        let qr = mfa_totp::qr_png_data_uri(&url).unwrap_or_default();
        let now: chrono::DateTime<chrono::FixedOffset> = store::now();
        Ok(Response::new(authv1::StartEnrollMethodResponse {
            result: Some(authv1::start_enroll_method_response::Result::Totp(
                authv1::TotpResult {
                    secret,
                    otp_auth_url: url,
                    qr_code_data_uri: qr,
                },
            )),
            expires_at: ts_to_proto(
                now + chrono::Duration::seconds(mfa_totp::MFA_CHALLENGE_TTL_SECS),
            ),
            operation_id: op_id,
        }))
    }

    /// The enroll confirm: the first code proves the app imported the
    /// secret; the peek keeps the operation alive across wrong tries,
    /// the delete only lands after the factor row does.
    async fn confirm_enroll_method(
        &self,
        request: Request<authv1::ConfirmEnrollMethodRequest>,
    ) -> Result<Response<authv1::ConfirmEnrollMethodResponse>, Status> {
        let uid = context::operator_of(&request)?;
        let tid = context::request_tenant_of(&request);
        let req = request.into_inner();
        if req.method != authv1::MfaMethod::Totp as i32 {
            return Err(bad("only TOTP is supported"));
        }
        let code = match req.credential.as_ref() {
            Some(authv1::confirm_enroll_method_request::Credential::TotpCode(c)) => c.as_str(),
            _ => return Err(bad("totp_code required")),
        };
        let cache = self.cache();
        let ctx = cache
            .peek_enroll_challenge(&req.operation_id)
            .await
            .ok_or_else(|| bad("invalid or expired enroll operation"))?;
        if ctx.tenant_id != tid || ctx.user_id != uid {
            return Err(forbidden("enroll operation user mismatch"));
        }
        let secret = mfa_totp::base32_decode(&ctx.secret)
            .ok_or_else(|| Status::internal("enroll secret undecodable"))?;
        let now = chrono::Utc::now().timestamp();
        if !mfa_totp::totp_verify(code, &secret, now) {
            return Ok(Response::new(authv1::ConfirmEnrollMethodResponse {
                success: false,
                ..Default::default()
            }));
        }
        let display = req.display.unwrap_or_default();
        let factor_id =
            repo::create_totp_factor(&self.state.db, tid, uid, &ctx.secret, &display).await?;
        cache.delete_enroll_challenge(&req.operation_id).await;
        Ok(Response::new(authv1::ConfirmEnrollMethodResponse {
            success: true,
            credential_id: factor_id.to_string(),
        }))
    }

    /// The disable face: self-service by credential id or by method;
    /// naming another user is the platform-admin rescue reset.
    async fn disable_mfa(
        &self,
        request: Request<authv1::DisableMfaRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let uid = context::operator_of(&request)?;
        let tid = context::request_tenant_of(&request);
        let req = request.into_inner();
        let parse_factor_id = |s: &str| -> Result<i64, Status> {
            s.parse::<i64>().map_err(|_| bad("invalid credential id"))
        };
        // The rescue path: another user's factors, platform admin only.
        if req.user_id.unwrap_or(0) != 0 && req.user_id.unwrap_or(0) != uid {
            // The operator rides tenant 0 only as the platform admin.
            if tid != 0 {
                return Err(forbidden("only platform admin can reset mfa for others"));
            }
            if let Some(credential_id) = req.credential_id.as_deref().filter(|s| !s.is_empty()) {
                let factor_id = parse_factor_id(credential_id)?;
                let (ftid, fuid) = repo::get_factor_by_id(&self.state.db, factor_id)
                    .await?
                    .ok_or_else(|| Status::not_found("mfa credential not found"))?;
                if fuid != req.user_id.unwrap_or(0) {
                    return Err(Status::not_found("mfa credential not found"));
                }
                if !repo::delete_for_user(&self.state.db, ftid, fuid, factor_id).await? {
                    return Err(Status::not_found("mfa credential not found"));
                }
                return Ok(Response::new(pbjson_types::Empty {}));
            }
            let method = method_spelling(req.method.unwrap_or(0))?;
            let (ftid, fuid) =
                repo::find_first_by_user(&self.state.db, req.user_id.unwrap_or(0), method)
                    .await?
                    .ok_or_else(|| Status::not_found("mfa credential not found"))?;
            repo::delete_all_by_user_method(&self.state.db, ftid, fuid, method).await?;
            return Ok(Response::new(pbjson_types::Empty {}));
        }
        // The self path: by method (all factors) or by credential id.
        let by_method = req
            .credential_id
            .as_deref()
            .filter(|s| !s.is_empty())
            .is_none();
        if by_method {
            let method = method_spelling(req.method.unwrap_or(0))?;
            let n = repo::delete_all_by_user_method(&self.state.db, tid, uid, method).await?;
            if n == 0 {
                return Err(Status::not_found("mfa credential not found"));
            }
            return Ok(Response::new(pbjson_types::Empty {}));
        }
        let factor_id = parse_factor_id(req.credential_id.as_deref().unwrap_or_default())?;
        if !repo::delete_for_user(&self.state.db, tid, uid, factor_id).await? {
            return Err(Status::not_found("mfa credential not found"));
        }
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The per-device revoke (the ownership guard rides the delete).
    async fn revoke_mfa_device(
        &self,
        request: Request<authv1::RevokeMfaDeviceRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let uid = context::operator_of(&request)?;
        let tid = context::request_tenant_of(&request);
        let req = request.into_inner();
        let factor_id = req
            .credential_id
            .parse::<i64>()
            .map_err(|_| bad("invalid credential id"))?;
        if !repo::delete_for_user(&self.state.db, tid, uid, factor_id).await? {
            return Err(Status::not_found("mfa credential not found"));
        }
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The login challenge face — free of the operator header: the
    /// challenge context carries who this is. Peek (retries survive a
    /// wrong code), re-verify against the factor's decrypted secret,
    /// void on the failure cap, adjudicate atomically, then issue the
    /// real pair.
    async fn verify_mfa_challenge(
        &self,
        request: Request<authv1::VerifyMfaChallengeRequest>,
    ) -> Result<Response<authv1::LoginResponse>, Status> {
        let req = request.into_inner();
        let code = challenge_totp_code(&req).ok_or_else(|| bad("totp_code required"))?;
        let cache = self.cache();
        let Some(ctx) = cache.peek_login_challenge(&req.operation_id).await else {
            return Err(bad("invalid or expired mfa operation"));
        };
        let payload = ctx.payload;
        let uid = Self::payload_i64(&payload, "uid");
        let tid = Self::payload_i64(&payload, "tid");
        let username = Self::payload_username(&payload);
        let Some((factor_id, secret_text)) =
            repo::find_enabled_totp_secret(&self.state.db, tid, uid).await
        else {
            // The factor vanished mid-challenge (revoked/reset): void the
            // challenge — the only way through is the password flow again.
            cache.consume_login_challenge(&req.operation_id).await;
            return Err(Status::permission_denied("mfa verification failed"));
        };
        // The stored form is the base32 text; the HMAC rides the decoded
        // key bytes (the confirm path decodes the same way).
        let Some(secret) = mfa_totp::base32_decode(&secret_text) else {
            cache.consume_login_challenge(&req.operation_id).await;
            return Err(Status::permission_denied("mfa verification failed"));
        };
        let now = chrono::Utc::now().timestamp();
        if !mfa_totp::totp_verify(code, &secret, now) {
            if cache.record_login_failure(&req.operation_id).await {
                return Err(Status::permission_denied(
                    "too many invalid mfa attempts, please login again",
                ));
            }
            return Err(Status::permission_denied("invalid mfa code"));
        }
        if !cache.take_login_challenge_atomic(&req.operation_id).await {
            return Err(Status::permission_denied("mfa challenge already consumed"));
        }
        repo::update_last_used(&self.state.db, tid, uid, factor_id).await;
        let client = crate::token::ClientKind::from_i32(ctx.client_type);
        let (access, refresh) = self.issue_pair(client, uid, tid, &username).await?;
        Ok(Response::new(self.login_response(client, access, refresh)))
    }

    /// The reference ships this round without the challenge start (the
    /// TOTP face needs no server-side start; the login flow opens the
    /// challenge directly).
    async fn start_mfa_challenge(
        &self,
        _request: Request<authv1::StartMfaChallengeRequest>,
    ) -> Result<Response<authv1::StartMfaChallengeResponse>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }

    async fn generate_backup_codes(
        &self,
        _request: Request<authv1::GenerateBackupCodesRequest>,
    ) -> Result<Response<authv1::GenerateBackupCodesResponse>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }

    async fn list_backup_codes(
        &self,
        _request: Request<authv1::ListBackupCodesRequest>,
    ) -> Result<Response<authv1::ListBackupCodesResponse>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }
}
