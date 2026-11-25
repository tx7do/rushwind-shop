//! Config loading — parses the embedded yaml defaults
//! (`assets/data.yaml`, compiled into the binary) with env overrides
//! (`RUSHWIND_DATABASE_SOURCE` wins over the embedded default). The
//! bind address rides the same document.

use serde::Deserialize;

const DATA_YAML: &str = include_str!("../assets/data.yaml");

#[derive(Debug, Clone)]
pub struct Config {
    /// The Postgres source (the core service is the ONLY process
    /// touching the database).
    pub database_source: String,
    /// The domain gRPC face's bind address.
    pub grpc_addr: String,
    pub redis_addr: String,
    pub redis_password: String,
    /// HS256 shared secret (the core mints, the gateways verify).
    pub jwt_key: String,
    pub access_token_expires_secs: i64,
    pub refresh_token_expires_secs: i64,
    pub app_access_token_expires_secs: i64,
}

#[derive(Debug, Deserialize)]
struct DataFile {
    data: Option<DataSection>,
}

#[derive(Debug, Default, Deserialize)]
struct DataSection {
    #[serde(default)]
    database: Option<DatabaseSection>,
    #[serde(default)]
    grpc_addr: Option<String>,
    #[serde(default)]
    redis: Option<RedisSection>,
    #[serde(default)]
    jwt_key: Option<String>,
    #[serde(default)]
    access_token_expires: Option<i64>,
    #[serde(default)]
    refresh_token_expires: Option<i64>,
    #[serde(default)]
    app_access_token_expires: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
struct RedisSection {
    #[serde(default)]
    addr: String,
    #[serde(default)]
    password: String,
}

#[derive(Debug, Default, Deserialize)]
struct DatabaseSection {
    #[serde(default)]
    #[allow(dead_code)]
    driver: String,
    #[serde(default)]
    source: String,
}

impl Config {
    /// Embedded defaults, env overrides win.
    pub fn load() -> Result<Self, String> {
        let data: DataFile =
            serde_yaml::from_str(DATA_YAML).map_err(|e| format!("data.yaml: {e}"))?;
        let section = data.data.unwrap_or_default();
        let database_source = std::env::var("RUSHWIND_DATABASE_SOURCE")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| section.database.map(|d| d.source))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                "postgres://postgres:*Abcd123456@127.0.0.1:5432/rushwind_shop?sslmode=disable"
                    .to_string()
            });
        let grpc_addr = section
            .grpc_addr
            .unwrap_or_else(|| "0.0.0.0:10800".to_string());
        let redis = section.redis.unwrap_or_default();
        let redis_addr = std::env::var("RUSHWIND_REDIS_ADDR")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                if redis.addr.is_empty() {
                    "127.0.0.1:6379".to_string()
                } else {
                    redis.addr
                }
            });
        let jwt_key = std::env::var("jwt_signing_key")
            .ok()
            .filter(|s| !s.is_empty())
            .or(section.jwt_key)
            .unwrap_or_else(|| "dev_only_change_me_in_prod".to_string());
        Ok(Self {
            database_source,
            grpc_addr,
            redis_addr,
            redis_password: redis.password,
            jwt_key,
            access_token_expires_secs: section.access_token_expires.unwrap_or(5400),
            refresh_token_expires_secs: section.refresh_token_expires.unwrap_or(43200),
            app_access_token_expires_secs: section.app_access_token_expires.unwrap_or(900),
        })
    }
}
