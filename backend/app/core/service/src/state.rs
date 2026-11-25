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

        Ok(Self {
            cfg,
            db,
            redis: redis_conn,
            jwt,
            admin_tokens,
            app_tokens,
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
