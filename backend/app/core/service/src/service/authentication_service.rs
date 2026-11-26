//! The authentication service — the credential verification, the JWT
//! pair issuance over the Redis session stores, the captcha pair, and
//! the token administration face (the reference's
//! authentication_service.go, trimmed to the shop contract's surface:
//! the tenant/authority enrichment joins as the identity/permission
//! domains land).

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, AppState};
use store::entities::sys_users;

use shop_proto::proto::authentication::service::v1 as authv1;
use shop_proto::proto::authentication::service::v1::{
    login_request, BlockTokenRequest, BlockTokenResponse, ChangePasswordRequest,
    GenerateCaptchaResponse, GetAccessTokensRequest, GetAccessTokensResponse, GrantType,
    LoginRequest, LoginResponse, LogoutRequest, RegisterUserRequest, RegisterUserResponse,
    RevokeTokenByIdRequest, TokenType, UnblockTokenRequest, UserTokenPayload, ValidateTokenRequest,
    ValidateTokenResponse, VerifyCaptchaRequest, VerifyCaptchaResponse, WhoAmIResponse,
};

type ResponseResult<T> = Result<Response<T>, Status>;

/// The verified identity riding the token — the claims bag the gate
/// injects and the BFF's operator extraction reads.
pub struct Identity {
    pub user_id: i64,
    pub tenant_id: i64,
    pub username: String,
    pub jti: String,
}

pub struct AuthenticationService {
    pub state: Arc<AppState>,
}

impl AuthenticationService {
    fn tokens(&self, client: crate::token::ClientKind) -> &crate::token::TokenStore {
        self.state.tokens(client)
    }

    async fn user_row(&self, id: i64) -> Result<sys_users::Model, Status> {
        sys_users::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::unauthenticated("user not found"))
    }

    /// The credential check: username → the row → the stored hash via
    /// the engine's Argon2id. The wire carries the front-end's
    /// AES-encrypted password (the VITE_AES_KEY envelope, base64) —
    /// the decrypt runs first and falls back to the raw spelling so
    /// the plaintext probes (curl, the dev seeding) keep working; a
    /// plaintext-equal fallback exists ONLY for pre-migration dev rows
    /// (a PHC string never equals a raw password, so production hashes
    /// take the engine path alone).
    async fn verify_credential(
        &self,
        username: &str,
        password: &str,
    ) -> Result<sys_users::Model, Status> {
        let row = sys_users::Entity::find()
            .filter(sys_users::Column::Username.eq(username))
            .one(&self.state.db)
            .await
            .map_err(db_status)?;
        let Some(row) = row else {
            crate::crypto::dummy_verify();
            return Err(Status::unauthenticated("user not found"));
        };
        if row.status.as_deref() != Some("NORMAL") {
            crate::crypto::dummy_verify();
            return Err(Status::unauthenticated("user not found"));
        }
        // The front-end envelope first: base64(AES-128-CBC(key, iv=key,
        // PKCS7)). An undecryptable wire (a plaintext probe) keeps the
        // raw spelling.
        let plain = store::crypto::decrypt_login_credential(password)
            .unwrap_or_else(|_| password.to_string());
        let stored = row.password.as_deref().unwrap_or("");
        let ok = rushwind_password::PasswordHasher::verify(
            &rushwind_password_argon2::Argon2Hasher::new(),
            &plain,
            stored,
        )
        .unwrap_or(false)
            || (stored == plain && !plain.is_empty());
        if !ok {
            crate::crypto::dummy_verify();
            return Err(Status::unauthenticated("user not found"));
        }
        Ok(row)
    }

    async fn issue_pair(
        &self,
        client: crate::token::ClientKind,
        user_id: i64,
        tenant_id: i64,
        username: &str,
    ) -> Result<(String, Option<String>), Status> {
        use rushwind_authn::Authenticator as _;
        let store = self.tokens(client);
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
    ) -> LoginResponse {
        let store = self.tokens(client);
        LoginResponse {
            token_type: TokenType::Bearer as i32,
            access_token: access,
            refresh_token: refresh,
            expires_in: store.access_secs(),
            refresh_expires_in: Some(store.refresh_secs()),
            ..Default::default()
        }
    }

    async fn do_password(&self, req: &LoginRequest) -> Result<LoginResponse, Status> {
        let client = crate::token::ClientKind::from_i32(req.client_type.unwrap_or(0));
        let username = match &req.identifier {
            Some(login_request::Identifier::Username(name)) if !name.trim().is_empty() => {
                name.trim().to_string()
            }
            _ => return Err(bad("identifier.username required")),
        };
        let row = self
            .verify_credential(&username, req.password.as_deref().unwrap_or(""))
            .await?;
        // The MFA gate: a user riding an ENABLED TOTP factor gets no
        // token from the password step — an operation id instead, and
        // MfaService::verify_mfa_challenge issues the real pair.
        let tid = row.tenant_id.unwrap_or(0);
        if crate::data::mfa_factor_repo::has_enabled_totp(&self.state.db, tid, row.id).await? {
            let cache =
                crate::data::mfa_challenge_cache::MfaChallengeCache::new(self.state.redis.clone());
            let op_id = cache
                .set_login_challenge(
                    serde_json::json!({
                        "uid": row.id,
                        "tid": tid,
                        "sub": row.username,
                    }),
                    client as i32,
                )
                .await
                .map_err(Status::internal)?;
            return Ok(LoginResponse {
                token_type: TokenType::Bearer as i32,
                mfa_operation_id: Some(op_id),
                ..Default::default()
            });
        }
        let (access, refresh) = self
            .issue_pair(client, row.id, tid, &row.username.clone())
            .await?;
        Ok(self.login_response(client, access, refresh))
    }

    async fn do_refresh(&self, req: &LoginRequest) -> Result<LoginResponse, Status> {
        let client = crate::token::ClientKind::from_i32(req.client_type.unwrap_or(0));
        let refresh = req
            .refresh_token
            .as_deref()
            .filter(|t| !t.is_empty())
            .ok_or_else(|| bad("refresh_token required"))?;
        let user_id = req.user_id.ok_or_else(|| bad("user_id required"))?;
        let row = self.user_row(user_id).await?;
        // The presented RT pins one jti row: scan the user's RT keys for
        // the exact match, revoke it, and mint the fresh pair.
        let jti = self
            .tokens(client)
            .find_jti_of_refresh(user_id, refresh)
            .await
            .map_err(Status::internal)?
            .ok_or_else(|| Status::unauthenticated("refresh token invalid"))?;
        let rotated = self
            .tokens(client)
            .verify_and_revoke_pair(user_id, &jti, refresh)
            .await
            .map_err(Status::internal)?;
        if !rotated {
            return Err(Status::unauthenticated("refresh token invalid"));
        }
        let (access, refresh) = self
            .issue_pair(
                client,
                row.id,
                row.tenant_id.unwrap_or(0),
                &row.username.clone(),
            )
            .await?;
        Ok(self.login_response(client, access, refresh))
    }

    /// The claims → payload decode, plus the session stage (whitelist
    /// exact match + blacklist existence).
    async fn validate(&self, token: &str, client_type: i32) -> Result<(Identity, bool), Status> {
        use rushwind_authn::Authenticator as _;
        let client = crate::token::ClientKind::from_i32(client_type);
        let claims = self
            .state
            .jwt
            .authenticate(&[("authorization".to_string(), format!("Bearer {token}"))])
            .map_err(|_| Status::unauthenticated("invalid token"))?;
        let uid = claims
            .0
            .get("uid")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| Status::unauthenticated("invalid token"))?;
        let jti = claims
            .0
            .get("jti")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let username = claims
            .0
            .get("sub")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let tid = claims.0.get("tid").and_then(|v| v.as_i64()).unwrap_or(0);
        let valid = self
            .tokens(client)
            .is_valid_access_token(uid, &jti, token)
            .await;
        let blocked = self.tokens(client).is_blocked(&jti).await;
        Ok((
            Identity {
                user_id: uid,
                tenant_id: tid,
                username,
                jti,
            },
            valid && !blocked,
        ))
    }
}

#[async_trait::async_trait]
impl authv1::authentication_service_server::AuthenticationService for AuthenticationService {
    async fn login(&self, request: Request<LoginRequest>) -> ResponseResult<LoginResponse> {
        let req = request.into_inner();
        let resp = match GrantType::try_from(req.grant_type) {
            Ok(GrantType::Password) => self.do_password(&req).await?,
            Ok(GrantType::RefreshToken) => self.do_refresh(&req).await?,
            _ => return Err(bad("invalid grant type")),
        };
        Ok(Response::new(resp))
    }

    async fn logout(&self, request: Request<LogoutRequest>) -> ResponseResult<pbjson_types::Empty> {
        let req = request.into_inner();
        let client = crate::token::ClientKind::from_i32(req.client_type);
        if req.user_id == 0 {
            return Err(bad("invalid user id"));
        }
        self.tokens(client)
            .revoke_user_tokens(req.user_id)
            .await
            .map_err(Status::internal)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The self-service password change: the storefront posts the
    /// AES-encrypted old/new pair; the old one is verified with the
    /// same decrypt-first path as login (wrong old → unauthenticated),
    /// the new one is decrypted then hashed into the row.
    async fn change_password(
        &self,
        request: Request<ChangePasswordRequest>,
    ) -> ResponseResult<pbjson_types::Empty> {
        let req = request.into_inner();
        if req.user_id <= 0 {
            return Err(bad("user_id required"));
        }
        let row = sys_users::Entity::find_by_id(req.user_id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::unauthenticated("user not found"))?;
        // Decrypt-first (the wire carries the AES envelope); the raw
        // spelling stays a fallback for plaintext probes.
        let old_plain = store::crypto::decrypt_login_credential(&req.old_password)
            .unwrap_or_else(|_| req.old_password.clone());
        let new_plain = store::crypto::decrypt_login_credential(&req.new_password)
            .unwrap_or_else(|_| req.new_password.clone());
        let stored = row.password.as_deref().unwrap_or("");
        let old_ok = rushwind_password::PasswordHasher::verify(
            &rushwind_password_argon2::Argon2Hasher::new(),
            &old_plain,
            stored,
        )
        .unwrap_or(false)
            || (stored == old_plain && !old_plain.is_empty());
        if !old_ok {
            crate::crypto::dummy_verify();
            return Err(Status::unauthenticated("old password incorrect"));
        }
        let hash = rushwind_password::PasswordHasher::hash(
            &rushwind_password_argon2::Argon2Hasher::new(),
            &new_plain,
        )
        .map_err(|_| Status::internal("hash failed"))?;
        let mut active: sys_users::ActiveModel = row.into();
        active.password = Set(Some(hash));
        active.updated_at = Set(Some(store::now()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn register_user(
        &self,
        request: Request<RegisterUserRequest>,
    ) -> ResponseResult<RegisterUserResponse> {
        let req = request.into_inner();
        let username = req.username.trim();
        if username.is_empty() {
            return Err(bad("username required"));
        }
        // Decrypt-first (the storefront registers through the same AES
        // envelope as login); plaintext probes keep working.
        let plain = store::crypto::decrypt_login_credential(&req.password)
            .unwrap_or_else(|_| req.password.clone());
        let hash = rushwind_password::PasswordHasher::hash(
            &rushwind_password_argon2::Argon2Hasher::new(),
            &plain,
        )
        .map_err(|_| Status::internal("hash failed"))?;
        let taken = sys_users::Entity::find()
            .filter(sys_users::Column::Username.eq(username))
            .one(&self.state.db)
            .await
            .map_err(db_status)?;
        if taken.is_some() {
            return Err(Status::already_exists("username taken"));
        }
        let active = sys_users::ActiveModel {
            username: sea_orm::Set(username.to_string()),
            nickname: sea_orm::Set(Some(username.to_string())),
            email: sea_orm::Set(req.email.clone()),
            password: sea_orm::Set(Some(hash)),
            status: sea_orm::Set(Some("NORMAL".to_string())),
            created_at: sea_orm::Set(Some(store::now())),
            ..Default::default()
        };
        let row = active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(RegisterUserResponse { user_id: row.id }))
    }

    async fn refresh_token(&self, request: Request<LoginRequest>) -> ResponseResult<LoginResponse> {
        let req = request.into_inner();
        Ok(Response::new(self.do_refresh(&req).await?))
    }

    async fn validate_token(
        &self,
        request: Request<ValidateTokenRequest>,
    ) -> ResponseResult<ValidateTokenResponse> {
        let req = request.into_inner();
        let (identity, is_valid) = self.validate(&req.token, req.client_type).await?;
        let payload = UserTokenPayload {
            user_id: identity.user_id,
            tenant_id: Some(identity.tenant_id),
            username: Some(identity.username),
            jti: Some(identity.jti),
            ..Default::default()
        };
        Ok(Response::new(ValidateTokenResponse {
            payload: Some(payload),
            is_valid,
            is_blocked: !is_valid,
        }))
    }

    async fn get_access_tokens(
        &self,
        request: Request<GetAccessTokensRequest>,
    ) -> ResponseResult<GetAccessTokensResponse> {
        let req = request.into_inner();
        let client = crate::token::ClientKind::from_i32(req.client_type);
        let tokens = self
            .tokens(client)
            .access_tokens(req.user_id)
            .await
            .map_err(Status::internal)?;
        Ok(Response::new(GetAccessTokensResponse {
            access_tokens: tokens,
        }))
    }

    async fn revoke_token_by_id(
        &self,
        request: Request<RevokeTokenByIdRequest>,
    ) -> ResponseResult<pbjson_types::Empty> {
        let req = request.into_inner();
        // The contract carries no user_id — the blacklist is the
        // revocation (the gate checks it after the whitelist match).
        self.state
            .any_tokens()
            .block_token(&req.jti, 24 * 3600)
            .await
            .map_err(Status::internal)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn block_token(
        &self,
        request: Request<BlockTokenRequest>,
    ) -> ResponseResult<BlockTokenResponse> {
        let req = request.into_inner();
        // BlockTokenResponse is an empty message; the block is global
        // (jti-unique), TTL 24h. The target oneof carries either the raw
        // token or its jti — this phase blocks only a presented jti.
        let now = chrono::Utc::now().timestamp();
        let jti = match req.target {
            Some(authv1::block_token_request::Target::Jti(jti)) => jti,
            _ => return Err(bad("target.jti required")),
        };
        self.state
            .any_tokens()
            .block_token(&jti, 24 * 3600)
            .await
            .map_err(Status::internal)?;
        Ok(Response::new(BlockTokenResponse {
            blocked_until: Some(pbjson_types::Timestamp {
                seconds: now + 24 * 3600,
                nanos: 0,
            }),
        }))
    }

    async fn unblock_token(
        &self,
        request: Request<UnblockTokenRequest>,
    ) -> ResponseResult<pbjson_types::Empty> {
        let req = request.into_inner();
        let jti = match req.target {
            Some(authv1::unblock_token_request::Target::Jti(jti)) => jti,
            _ => return Err(bad("target.jti required")),
        };
        // The blacklist row itself: DEL rws:bl:{jti}.
        let mut conn = self.state.redis.clone();
        let _: () = redis::cmd("DEL")
            .arg(format!("rws:bl:{}", jti))
            .query_async(&mut conn)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn who_am_i(
        &self,
        _request: Request<pbjson_types::Empty>,
    ) -> ResponseResult<WhoAmIResponse> {
        // WhoAmI answers from the caller's own claims (the reference's
        // form reads the metadata the gate injected); over unary gRPC
        // the BFF forwards the verified claims as metadata. Phase:
        // the identity lands with the user-profile domain.
        Err(Status::unimplemented("requires caller claims metadata"))
    }

    async fn generate_captcha(
        &self,
        _request: Request<pbjson_types::Empty>,
    ) -> ResponseResult<GenerateCaptchaResponse> {
        // The shop contract's captcha pair: a 4-digit numeric code with
        // a 5-minute TTL, keyed by the returned captcha id. The SVG
        // rendering rides with the admin login page's landing.
        // Four decimal digits off the uuid crate's CSPRNG bytes.
        let uid_bytes = *uuid::Uuid::new_v4().as_bytes();
        let code: String = uid_bytes[..4]
            .iter()
            .map(|b| char::from(b'0' + (b % 10)))
            .collect();
        let id = uuid::Uuid::new_v4().to_string();
        let mut conn = self.state.redis.clone();
        let _: () = redis::cmd("SETEX")
            .arg(format!("rws:captcha:{id}"))
            .arg(300i64)
            .arg(&code)
            .query_async(&mut conn)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let image = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"96\" height=\"32\">             <rect width=\"100%\" height=\"100%\" fill=\"#f0f0f0\"/>             <text x=\"24\" y=\"22\" font-size=\"20\" fill=\"#333\">{code}</text></svg>"
        );
        Ok(Response::new(GenerateCaptchaResponse {
            captcha_id: id,
            image_base64: {
                use base64::Engine as _;
                base64::engine::general_purpose::STANDARD.encode(image.as_bytes())
            },
        }))
    }

    async fn verify_captcha(
        &self,
        request: Request<VerifyCaptchaRequest>,
    ) -> ResponseResult<VerifyCaptchaResponse> {
        let req = request.into_inner();
        let mut conn = self.state.redis.clone();
        let key = format!("rws:captcha:{}", req.captcha_id);
        let stored: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        // Single-use: verify deletes, right or wrong.
        let _: () = redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let ok = stored.as_deref() == Some(req.user_input.as_str());
        Ok(Response::new(VerifyCaptchaResponse { valid: ok }))
    }
}
