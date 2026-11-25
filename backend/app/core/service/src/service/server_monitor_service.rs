//! The server-monitor service — the process/host health projection
//! (the reference's server_monitor_service.go): the runtime stats (a
//! tokio face over the Go-runtime fields), the database pool health
//! (the live ping), and the host facts. A pure runtime read — no
//! tables.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::state::AppState;

use shop_proto::proto::server_monitor::service::v1 as smv1;

/// The process start instant, captured once at first read.
fn process_started_at() -> std::time::Instant {
    use std::sync::OnceLock;
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    *START.get_or_init(std::time::Instant::now)
}

pub struct ServerMonitorService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl smv1::server_monitor_service_server::ServerMonitorService for ServerMonitorService {
    async fn get(
        &self,
        _request: Request<smv1::GetServerMonitorRequest>,
    ) -> Result<Response<smv1::ServerMonitorInfo>, Status> {
        // The runtime face: the tokio metrics over the Go-runtime
        // fields (the alive-task count as the goroutine gauge, the
        // sysinfo heap numbers as the alloc/sys pair).
        let alive_tasks = tokio::runtime::Handle::try_current()
            .map(|h| h.metrics().num_alive_tasks() as u32)
            .unwrap_or(0);
        let mut system = sysinfo::System::new();
        system.refresh_memory();
        let alloc_bytes = (system.used_memory() * 1024) as i64;
        let sys_bytes = (system.total_memory() * 1024) as i64;
        let uptime = process_started_at().elapsed().as_secs() as i64;

        // The host face.
        let num_cpu = system.cpus().len() as u32;
        let hostname = sysinfo::System::host_name();

        // The database face: the live ping through the pool.
        let (ping_ok, ping_error) = match self.state.db.ping().await {
            Ok(()) => (true, None),
            Err(e) => (false, Some(format!("{e}"))),
        };

        Ok(Response::new(smv1::ServerMonitorInfo {
            go: Some(smv1::GoRuntimeInfo {
                version: Some(env!("CARGO_PKG_RUST_VERSION").to_string()),
                num_goroutine: Some(alive_tasks),
                mem_alloc_bytes: Some(alloc_bytes),
                mem_sys_bytes: Some(sys_bytes),
                gc_cycles: None,
                uptime_seconds: Some(uptime),
                started_at: None,
            }),
            database: Some(smv1::DatabaseInfo {
                driver: Some("postgres".to_string()),
                ping_ok: Some(ping_ok),
                ping_error,
                max_open_connections: Some(25),
                open_connections: None,
                in_use_connections: None,
                idle_connections: None,
            }),
            host: Some(smv1::HostInfo {
                os: Some(std::env::consts::OS.to_string()),
                arch: Some(std::env::consts::ARCH.to_string()),
                num_cpu: Some(num_cpu),
                hostname,
                ..Default::default()
            }),
            collected_at: Some(crate::state::ts_to_proto(store::now()).unwrap_or(
                pbjson_types::Timestamp::default(),
            )),
        }))
    }
}
