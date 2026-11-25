//! Config loading — parses the embedded yaml defaults
//! (`assets/data.yaml`, `assets/auth.yaml`, compiled into the binary)
//! with env overrides (`RUSHWIND_CORE_ADDR`, plus the shared secret
//! through `jwt_signing_key` — the same env var name the Go
//! reference's `${jwt_signing_key:...}` placeholder resolves). The
//! server assembly document (`assets/server.yaml`) belongs to the
//! lifecycle assembler.

use rushwind_bootstrap::DurationWire;
use serde::Deserialize;

const DATA_YAML: &str = include_str!("../assets/data.yaml");
const AUTH_YAML: &str = include_str!("../assets/auth.yaml");

/// Which client section of auth.yaml this deployment eats — the admin
/// BFF reads `admin`, the app BFF reads `app` (mirroring the reference's
/// authenticator.yaml).
pub const AUTH_SECTION: &str = "admin";

#[derive(Debug, Clone)]
pub struct Config {
    /// The core domain service's gRPC endpoint (static dial — the
    /// reference resolves it via registry).
    pub core_addr: String,
    /// The verification engine's HS256 shared secret (verify + mint) —
    /// the client section's key or the `jwt_signing_key` env override.
    pub jwt_key: String,
    pub access_token_expires_secs: i64,
    pub refresh_token_expires_secs: i64,
}

#[derive(Debug, Deserialize)]
struct DataFile {
    data: Option<DataSection>,
}

#[derive(Debug, Default, Deserialize)]
struct DataSection {
    #[serde(default)]
    core_addr: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuthFile {
    authenticator: Option<AuthenticatorSection>,
}

#[derive(Debug, Default, Deserialize)]
struct AuthenticatorSection {
    #[serde(default)]
    admin: Option<ClientSection>,
    #[serde(default)]
    app: Option<ClientSection>,
}

#[derive(Debug, Default, Deserialize)]
struct ClientSection {
    #[serde(default)]
    #[allow(dead_code)]
    method: String,
    #[serde(default)]
    key: String,
    #[serde(default)]
    access_token_expires: Option<DurationWire>,
    #[serde(default)]
    refresh_token_expires: Option<DurationWire>,
}

impl Config {
    /// Embedded defaults, env overrides win.
    pub fn load() -> Result<Self, String> {
        let data: DataFile = serde_yaml::from_str(DATA_YAML)
            .map_err(|e| format!("data.yaml: {e}"))?;
        let auth: AuthFile = serde_yaml::from_str(AUTH_YAML)
            .map_err(|e| format!("auth.yaml: {e}"))?;

        let section = auth
            .authenticator
            .and_then(|a| match AUTH_SECTION {
                "app" => a.app,
                _ => a.admin,
            })
            .unwrap_or_default();

        let core_addr = std::env::var("RUSHWIND_CORE_ADDR")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| data.data.and_then(|d| d.core_addr))
            .unwrap_or_else(|| "http://127.0.0.1:10800".to_string());
        let jwt_key = std::env::var("jwt_signing_key")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or(section.key);
        let to_secs = |d: Option<DurationWire>| -> i64 {
            d.map(|d| d.0.as_secs() as i64).unwrap_or(5400)
        };

        Ok(Self {
            core_addr,
            jwt_key,
            access_token_expires_secs: to_secs(section.access_token_expires),
            refresh_token_expires_secs: to_secs(section.refresh_token_expires),
        })
    }
}
