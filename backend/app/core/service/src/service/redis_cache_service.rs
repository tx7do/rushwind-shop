//! The redis-cache monitor service — the live Redis INFO projection
//! (the reference's redis_cache_monitor_service.go): the server/stats
//! sections as key-value listings, the keyspace size, and the slowlog
//! tail. A pure runtime read — no tables.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::state::AppState;

use shop_proto::proto::redis_cache::service::v1 as rcv1;

/// Splits one INFO section's body into the key-value entries (skipping
/// the informational comments).
fn parse_section(body: &str) -> Vec<rcv1::InfoEntry> {
    body.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (k, v) = line.split_once(':')?;
            Some(rcv1::InfoEntry {
                key: k.to_string(),
                value: v.to_string(),
            })
        })
        .collect()
}

pub struct RedisCacheMonitorService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl rcv1::redis_cache_monitor_service_server::RedisCacheMonitorService
    for RedisCacheMonitorService
{
    async fn get(
        &self,
        _request: Request<rcv1::GetRedisCacheMonitorRequest>,
    ) -> Result<Response<rcv1::RedisCacheMonitorInfo>, Status> {
        let mut conn = self.state.redis.clone();

        // The sections the storefront's monitor view renders.
        let mut sections = Vec::new();
        for name in ["server", "stats", "keyspace"] {
            let body: String = redis::cmd("INFO")
                .arg(name)
                .query_async(&mut conn)
                .await
                .map_err(|e| Status::internal(format!("redis info: {e}")))?;
            sections.push(rcv1::InfoSection {
                name: name.to_string(),
                entries: parse_section(&body),
            });
        }

        // The keyspace size (the logical DB the sessions/queues ride).
        let db_size: i64 = redis::cmd("DBSIZE")
            .query_async(&mut conn)
            .await
            .map_err(|e| Status::internal(format!("redis dbsize: {e}")))?;

        // The slowlog tail (the last 10 entries: id, timestamp,
        // duration, command args, client).
        let raw: Vec<(i64, i64, i64, Vec<String>, Option<String>, Option<String>)> =
            redis::cmd("SLOWLOG")
                .arg("GET")
                .arg(10)
                .query_async(&mut conn)
                .await
                .map_err(|e| Status::internal(format!("redis slowlog: {e}")))?;
        let slowlog = raw
            .into_iter()
            .map(|(id, ts, dur, args, addr, name)| rcv1::SlowLogEntry {
                id,
                created_at: Some(pbjson_types::Timestamp {
                    seconds: ts,
                    nanos: 0,
                }),
                duration_usec: dur,
                args,
                client_addr: addr.unwrap_or_default(),
                client_name: name.unwrap_or_default(),
            })
            .collect();

        Ok(Response::new(rcv1::RedisCacheMonitorInfo {
            sections,
            db_size,
            slowlog,
        }))
    }
}
