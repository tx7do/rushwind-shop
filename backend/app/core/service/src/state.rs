//! The core application state: the database (the ONLY process touching
//! it), the Redis session stores, and the HS256 mint/verify engine.

use redis::aio::ConnectionManager;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

use crate::config::Config;
use crate::token::{ClientKind, TokenStore};

pub struct AppState {
    pub cfg: Config,
    pub db: DatabaseConnection,
    pub redis: ConnectionManager,
    /// The mint/verify engine — HS256, one shared secret (the gateways
    /// verify the same secret; the core mints).
    pub jwt: rushwind_authn_jwt::JwtAuthenticator,
    /// The per-client session stores (admin ct=0, app ct=1).
    pub admin_tokens: TokenStore,
    pub app_tokens: TokenStore,
    /// The object store (the config-selected S3 engine); `None` when
    /// the config carries no oss section — the storage faces answer
    /// FAILED_PRECONDITION then.
    pub oss: Option<std::sync::Arc<dyn rushwind_oss::ObjectStorage>>,
}

impl AppState {
    pub fn tokens(&self, client: ClientKind) -> &TokenStore {
        match client {
            ClientKind::Admin => &self.admin_tokens,
            ClientKind::App => &self.app_tokens,
        }
    }

    /// The client-agnostic face (the blacklist is jti-global).
    pub fn any_tokens(&self) -> &TokenStore {
        &self.admin_tokens
    }

    pub async fn connect(cfg: Config) -> Result<Self, String> {
        let mut opts = ConnectOptions::new(cfg.database_source.clone());
        opts.max_connections(25)
            .min_connections(5)
            .connect_timeout(std::time::Duration::from_secs(10));
        let db = Database::connect(opts)
            .await
            .map_err(|e| format!("db connect: {e}"))?;

        let redis_source = if cfg.redis_password.is_empty() {
            format!("redis://{}/", cfg.redis_addr)
        } else {
            format!("redis://:{}@{}/", cfg.redis_password, cfg.redis_addr)
        };
        let client = redis::Client::open(redis_source).map_err(|e| format!("redis url: {e}"))?;
        let redis_conn = client
            .get_connection_manager()
            .await
            .map_err(|e| format!("redis connect: {e}"))?;

        let jwt = rushwind_authn_jwt::JwtAuthenticator::new(
            rushwind_authn_jwt::JwtOptions::new()
                .with_algorithm("HS256")
                .map_err(|e| format!("jwt algorithm: {e:?}"))?
                .with_key(cfg.jwt_key.as_bytes()),
        );

        let admin_tokens = TokenStore::new(
            redis_conn.clone(),
            ClientKind::Admin,
            cfg.access_token_expires_secs,
            cfg.refresh_token_expires_secs,
        );
        let app_tokens = TokenStore::new(
            redis_conn.clone(),
            ClientKind::App,
            cfg.app_access_token_expires_secs,
            0,
        );

        let oss: Option<std::sync::Arc<dyn rushwind_oss::ObjectStorage>> = match &cfg.oss {
            Some(o) => {
                let engine = rushwind_oss_s3::S3Storage::new(rushwind_oss::StorageConfig {
                    endpoint: o.endpoint.clone(),
                    region: o.region.clone(),
                    access_key: o.access_key.clone(),
                    secret_key: o.secret_key.clone(),
                    token: None,
                    use_ssl: o.use_ssl,
                    force_path_style: o.force_path_style,
                    bucket: o.bucket.clone(),
                })
                .map_err(|e| format!("oss engine: {e:?}"))?;
                // Best-effort bucket setup (idempotent against an
                // existing bucket) — an empty MinIO otherwise refuses
                // the first put.
                if let Err(e) = engine.create_bucket().await {
                    tracing::warn!("oss bucket setup: {e:?} (continuing)");
                }
                Some(std::sync::Arc::new(engine) as _)
            }
            None => None,
        };

        Ok(Self {
            cfg,
            db,
            redis: redis_conn,
            jwt,
            admin_tokens,
            app_tokens,
            oss,
        })
    }
}

/// The DB failure → INTERNAL mapping the repos' `.map_err` rides.
pub fn db_status(e: sea_orm::DbErr) -> tonic::Status {
    tonic::Status::internal(format!("db: {e}"))
}

/// A bad-request Status.
pub fn bad(message: &str) -> tonic::Status {
    tonic::Status::invalid_argument(message.to_string())
}

/// A permission-denied Status.
pub fn forbidden(message: &str) -> tonic::Status {
    tonic::Status::permission_denied(message.to_string())
}

/// A not-found Status.
pub fn not_found(message: &str) -> tonic::Status {
    tonic::Status::not_found(message.to_string())
}

/// timestamptz → protojson Timestamp.
pub fn ts_to_proto(
    value: chrono::DateTime<chrono::FixedOffset>,
) -> Option<pbjson_types::Timestamp> {
    Some(pbjson_types::Timestamp {
        seconds: value.timestamp(),
        nanos: value.timestamp_subsec_nanos() as i32,
    })
}

pub type StatusResult<T> = Result<T, tonic::Status>;

/// NOT_FOUND with the resource label (the repo-layer form).
pub fn not_found_status(what: &str) -> tonic::Status {
    tonic::Status::not_found(format!("{what} not found"))
}

/// protojson Timestamp → timestamptz (the entity column form).
pub fn ts_from_proto(v: pbjson_types::Timestamp) -> chrono::DateTime<chrono::FixedOffset> {
    use chrono::TimeZone as _;
    chrono::Utc
        .timestamp_opt(v.seconds, v.nanos.max(0) as u32)
        .single()
        .unwrap_or_else(chrono::Utc::now)
        .fixed_offset()
}
