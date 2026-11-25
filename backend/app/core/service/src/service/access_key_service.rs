//! The access-key face (the reference's access_key_service.go) — the
//! machine credentials: CRUD with the secret's one-time reveal, the
//! rotation (ResetSecret), and the unauthenticated token exchange
//! (IssueToken — the AK identifies, the SK proves, the ledger row's
//! SHA-256 digest never carries the plaintext). The exchange mints a
//! machine identity (uid 0, the `ak:` subject, the machine role) the
//! same HS256 engine the user tokens ride.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use sha2::{Digest, Sha256};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_from_proto, ts_to_proto, AppState};
use store::entities::sys_access_keys;
use store::paging::fetch_paged;

use shop_proto::proto::access_key::service::v1 as akv1;
use shop_proto::proto::pagination::PagingRequest;

fn sha256_hex(text: &str) -> String {
    let sum = Sha256::digest(text.as_bytes());
    sum.iter().map(|b| format!("{b:02x}")).collect()
}

/// The AK/SK draws — hex stems off UUIDv4 pairs (unpredictable, the
/// same draw class the refresh token rides).
fn new_access_key() -> String {
    format!(
        "AK{}{}",
        uuid::Uuid::new_v4().simple(),
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    )
}

fn new_secret() -> String {
    format!(
        "SK{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn status_name(v: i32) -> Option<&'static str> {
    Some(match akv1::access_key::Status::try_from(v).ok()? {
        akv1::access_key::Status::Off => "OFF",
        _ => "ON",
    })
}

fn status_proto(name: Option<&str>) -> Option<i32> {
    Some(match name? {
        "OFF" => akv1::access_key::Status::Off as i32,
        _ => akv1::access_key::Status::On as i32,
    })
}

fn key_proto(r: sys_access_keys::Model) -> akv1::AccessKey {
    akv1::AccessKey {
        id: Some(r.id),
        name: r.name,
        access_key: r.access_key,
        status: status_proto(r.status.as_deref()),
        expires_at: r.expires_at.and_then(ts_to_proto),
        last_used_at: r.last_used_at.and_then(ts_to_proto),
        tenant_id: r.tenant_id,
        created_by: r.created_by,
        updated_by: r.updated_by,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

async fn by_id(
    db: &sea_orm::DatabaseConnection,
    id: i64,
) -> Result<sys_access_keys::Model, Status> {
    sys_access_keys::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(db_status)?
        .ok_or_else(|| Status::not_found("access key not found"))
}

pub struct AccessKeyService {
    pub state: Arc<AppState>,
}

impl AccessKeyService {
    /// The mint-and-record both Create and ResetSecret share: the SK
    /// plaintext rides the response once; only the digest lands.
    fn mint(&self) -> (String, String) {
        let secret = new_secret();
        let digest = sha256_hex(&secret);
        (secret, digest)
    }
}

#[async_trait::async_trait]
impl akv1::access_key_service_server::AccessKeyService for AccessKeyService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<akv1::ListAccessKeyResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sys_access_keys::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(akv1::ListAccessKeyResponse {
            items: rows.into_iter().map(key_proto).collect(),
            total: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<akv1::GetAccessKeyRequest>,
    ) -> Result<Response<akv1::AccessKey>, Status> {
        let req = request.into_inner();
        let Some(akv1::get_access_key_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        Ok(Response::new(key_proto(by_id(&self.state.db, id).await?)))
    }

    async fn create(
        &self,
        request: Request<akv1::CreateAccessKeyRequest>,
    ) -> Result<Response<akv1::CreateAccessKeyResponse>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let (secret, secret_hash) = self.mint();
        let active = sys_access_keys::ActiveModel {
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            name: Set(data.name),
            access_key: Set(Some(new_access_key())),
            secret_hash: Set(Some(secret_hash)),
            status: Set(Some("ON".to_string())),
            expires_at: Set(data.expires_at.map(ts_from_proto)),
            created_by: Set(data.created_by),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        let row = active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(akv1::CreateAccessKeyResponse {
            data: Some(key_proto(row)),
            secret,
        }))
    }

    async fn update(
        &self,
        request: Request<akv1::UpdateAccessKeyRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let existing = by_id(&self.state.db, req.id).await?;
        let mut active: sys_access_keys::ActiveModel = existing.into();
        if let Some(v) = data.name {
            active.name = Set(Some(v));
        }
        if let Some(v) = data.status {
            active.status = Set(status_name(v).map(String::from));
        }
        if let Some(v) = data.expires_at {
            active.expires_at = Set(Some(ts_from_proto(v)));
        }
        if let Some(v) = data.updated_by {
            active.updated_by = Set(Some(v));
        }
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<akv1::DeleteAccessKeyRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        if req.id == 0 {
            return Err(bad("id required"));
        }
        sys_access_keys::Entity::delete_by_id(req.id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The rotation: a new SK digest immediately blocks the old SK's
    /// exchanges; tokens already issued stay valid until they expire.
    async fn reset_secret(
        &self,
        request: Request<akv1::ResetAccessKeySecretRequest>,
    ) -> Result<Response<akv1::CreateAccessKeyResponse>, Status> {
        let req = request.into_inner();
        let id = if req.id != 0 {
            req.id
        } else {
            return Err(bad("id is required"));
        };
        let (secret, secret_hash) = self.mint();
        let existing = by_id(&self.state.db, id).await?;
        let mut active: sys_access_keys::ActiveModel = existing.into();
        active.secret_hash = Set(Some(secret_hash));
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(akv1::CreateAccessKeyResponse {
            data: None,
            secret,
        }))
    }

    /// The unauthenticated exchange (the machine-identity face): the
    /// AK resolves, the SK's digest matches in constant time, the
    /// status and expiry gates answer, and a machine token (uid 0,
    /// `ak:` subject, machine role) rides out.
    async fn issue_token(
        &self,
        request: Request<akv1::IssueTokenRequest>,
    ) -> Result<Response<akv1::IssueTokenResponse>, Status> {
        use rushwind_authn::Authenticator as _;
        let req = request.into_inner();
        if req.access_key.is_empty() || req.secret.is_empty() {
            return Err(bad("access key and secret are required"));
        }
        let row = sys_access_keys::Entity::find()
            .filter(sys_access_keys::Column::AccessKey.eq(req.access_key.clone()))
            .one(&self.state.db)
            .await
            .map_err(db_status)?;
        // The existence blind: the not-found path answers the same
        // message as the digest mismatch.
        let Some(row) = row else {
            return Err(bad("invalid access key or secret"));
        };
        if row.status.as_deref() != Some("ON") {
            return Err(bad("access key is disabled"));
        }
        if let Some(expires) = row.expires_at {
            if expires < chrono::DateTime::<chrono::FixedOffset>::from(store::now()) {
                return Err(bad("access key is expired"));
            }
        }
        let given = sha256_hex(&req.secret);
        let stored = row.secret_hash.clone().unwrap_or_default();
        if !constant_time_eq(given.as_bytes(), stored.as_bytes()) {
            return Err(bad("invalid access key or secret"));
        }
        // Best-effort last-used stamp (the exchange's only write).
        let mut active: sys_access_keys::ActiveModel = row.clone().into();
        active.last_used_at = Set(Some(store::now().into()));
        let _ = active.update(&self.state.db).await;

        let store = self.state.tokens(crate::token::ClientKind::Admin);
        let now = chrono::Utc::now().timestamp();
        let mut claims = serde_json::Map::new();
        claims.insert("sub".into(), format!("ak:{}", req.access_key).into());
        claims.insert("uid".into(), 0.into());
        claims.insert("tid".into(), row.tenant_id.unwrap_or(0).into());
        claims.insert("roles".into(), serde_json::json!(["machine"]));
        claims.insert("iat".into(), now.into());
        claims.insert("exp".into(), (now + store.access_secs()).into());
        let access = self
            .state
            .jwt
            .create_identity(&rushwind_authn::AuthClaims(claims))
            .map_err(|_| Status::internal("issue token failed"))?;
        Ok(Response::new(akv1::IssueTokenResponse {
            access_token: access,
            expires_in: store.access_secs() as u32,
            token_type: "bearer".to_string(),
        }))
    }
}

/// The length-constant comparison the SK digest check rides.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
