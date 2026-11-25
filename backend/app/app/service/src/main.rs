//! The app BFF service assembly entry: loads the embedded config
//! defaults (env overrides win), builds the HS256 engine (verify + mint
//! share the secret), dials the core domain service, and hands the
//! REST edge (:10700) to the config-driven lifecycle assembler (the
//! embedded server document, one `App`, one shutdown path).

use std::sync::Arc;

use app_service::config::Config;
use app_service::server::rest;
use app_service::state::AppState;
use rushwind_bootstrap::Bootstrap;
use rushwind_transport::StopSignal;

/// The server assembly document, compiled into the binary.
const SERVER_YAML: &str = include_str!("../assets/server.yaml");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::load()?;
    let verifier = rushwind_authn_jwt::JwtAuthenticator::new(
        rushwind_authn_jwt::JwtOptions::new()
            .with_algorithm("HS256")
            .map_err(|e| format!("jwt algorithm: {e:?}"))?
            .with_key(cfg.jwt_key.as_bytes()),
    );
    let authenticator: Arc<dyn rushwind_authn::Authenticator> = Arc::new(verifier);

    let state = Arc::new(AppState::connect(cfg, authenticator).await?);

    let assembler = Bootstrap::from_yaml_str(SERVER_YAML)?
        .route_pack("app-surface", rest::pack(Arc::clone(&state)));

    let booted = assembler.build().await?;
    // The lifecycle owns the OS-signal shutdown path internally; the
    // external signal here stays unfired.
    let external = StopSignal::new();
    booted.app.run(external).await?;
    Ok(())
}
