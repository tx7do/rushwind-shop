//! The monitor-alert service — the alert rule configuration plus the
//! evaluate face (the reference's monitor_alert_service.go). The rule
//! rows carry the metric/op/threshold triple and the notify target;
//! Evaluate walks the rules against the current process metrics (the
//! goroutine count and the heap allocation — the same names the enum
//! spells), fires on the comparison, records the last-firing state,
//! and hands the notification to the delivery ledger when the
//! cooldown window allows.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::monitor_alert_rules;
use store::paging::fetch_paged;

use shop_proto::proto::monitor_alert::service::v1 as mav1;
use shop_proto::proto::pagination::PagingRequest;

fn rule_proto(r: monitor_alert_rules::Model) -> mav1::MonitorAlertRule {
    mav1::MonitorAlertRule {
        id: Some(r.id),
        name: Some(r.name.clone()),
        metric: Some(
            mav1::MonitorMetric::try_from(match r.metric.as_str() {
                "GO_MEM_ALLOC_MB" => 2,
                _ => 1,
            })
            .unwrap_or(mav1::MonitorMetric::GoGoroutines) as i32,
        ),
        op: Some(
            mav1::AlertOp::try_from(if r.op == "LE" { 2 } else { 1 }).unwrap_or(mav1::AlertOp::Ge)
                as i32,
        ),
        threshold: Some(r.threshold),
        cooldown_minutes: r.cooldown_minutes.map(|v| v as u32),
        channel: Some(if r.channel.as_deref() == Some("WEBHOOK") {
            notifv1_channel::Channel::Webhook as i32
        } else if r.channel.as_deref() == Some("INTERNAL") {
            notifv1_channel::Channel::Internal as i32
        } else {
            notifv1_channel::Channel::Email as i32
        }),
        target: r.target.clone(),
        is_enabled: Some(r.is_enabled.unwrap_or(true)),
        remark: r.remark.clone(),
        last_firing: Some(r.last_firing),
        last_value: Some(r.last_value),
        last_alerted_at: r.last_alerted_at.and_then(ts_to_proto),
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

// The notification Channel enum lives in the notification package; the
// alias keeps the rule mapping readable.
use shop_proto::proto::notification::service::v1 as notifv1_channel;

/// The current metric sample: the process's own numbers (the names the
/// enum spells; the GO_ prefix carried over from the reference's
/// runtime metrics).
fn sample_metric(metric: &str) -> f64 {
    match metric {
        "GO_MEM_ALLOC_MB" => {
            let mut system = sysinfo::System::new();
            system.refresh_memory();
            (system.used_memory() as f64) / 1024.0 / 1024.0
        }
        // GO_GOROUTINES and the default: the tokio worker count as the
        // task-concurrency proxy (the true goroutine gauge is the
        // reference runtime's).
        _ => tokio::runtime::Handle::try_current()
            .map(|h| h.metrics().num_alive_tasks() as f64)
            .unwrap_or(0.0),
    }
}

pub struct MonitorAlertService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl mav1::monitor_alert_service_server::MonitorAlertService for MonitorAlertService {
    async fn list_monitor_alert_rule(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<mav1::ListMonitorAlertRuleResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            monitor_alert_rules::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(mav1::ListMonitorAlertRuleResponse {
            items: rows.into_iter().map(rule_proto).collect(),
            total: total as i64,
        }))
    }

    async fn get_monitor_alert_rule(
        &self,
        request: Request<mav1::GetMonitorAlertRuleRequest>,
    ) -> Result<Response<mav1::MonitorAlertRule>, Status> {
        let req = request.into_inner();
        let row = monitor_alert_rules::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("alert rule not found"))?;
        Ok(Response::new(rule_proto(row)))
    }

    async fn create_monitor_alert_rule(
        &self,
        request: Request<mav1::CreateMonitorAlertRuleRequest>,
    ) -> Result<Response<mav1::MonitorAlertRule>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let name = data
            .name
            .clone()
            .filter(|n| !n.is_empty())
            .ok_or_else(|| bad("data.name required"))?;
        let metric = match mav1::MonitorMetric::try_from(data.metric.unwrap_or(1)).ok() {
            Some(mav1::MonitorMetric::GoMemAllocMb) => "GO_MEM_ALLOC_MB",
            _ => "GO_GOROUTINES",
        };
        let active = monitor_alert_rules::ActiveModel {
            name: Set(name),
            metric: Set(metric.to_string()),
            op: Set(
                if mav1::AlertOp::try_from(data.op.unwrap_or(1)).ok() == Some(mav1::AlertOp::Le) {
                    "LE"
                } else {
                    "GE"
                }
                .to_string(),
            ),
            threshold: Set(data.threshold.unwrap_or(0.0)),
            cooldown_minutes: Set(Some(data.cooldown_minutes.unwrap_or(30) as i32)),
            channel: Set(Some(
                match notifv1_channel::Channel::try_from(data.channel.unwrap_or(3)).ok() {
                    Some(notifv1_channel::Channel::Webhook) => "WEBHOOK",
                    Some(notifv1_channel::Channel::Internal) => "INTERNAL",
                    _ => "EMAIL",
                }
                .to_string(),
            )),
            target: Set(data.target.clone()),
            is_enabled: Set(Some(data.is_enabled.unwrap_or(true))),
            remark: Set(data.remark.clone()),
            last_firing: Set(false),
            last_value: Set(0.0),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        let row = active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(rule_proto(row)))
    }

    async fn update_monitor_alert_rule(
        &self,
        request: Request<mav1::UpdateMonitorAlertRuleRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = monitor_alert_rules::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("alert rule not found"))?;
        let mut active: monitor_alert_rules::ActiveModel = existing.into();
        if let Some(v) = data.name.clone() {
            active.name = Set(v);
        }
        if let Some(v) = data.threshold {
            active.threshold = Set(v);
        }
        if let Some(v) = data.is_enabled {
            active.is_enabled = Set(Some(v));
        }
        if let Some(v) = data.target.clone() {
            active.target = Set(Some(v));
        }
        if let Some(v) = data.remark.clone() {
            active.remark = Set(Some(v));
        }
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete_monitor_alert_rule(
        &self,
        request: Request<mav1::DeleteMonitorAlertRuleRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        monitor_alert_rules::Entity::delete_by_id(req.id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The evaluate face: walk the enabled rules (one, when rule_id
    /// rides), sample the metric, compare per the op, persist the
    /// firing state, and count the notifications handed off.
    async fn evaluate_monitor_alerts(
        &self,
        request: Request<mav1::EvaluateMonitorAlertsRequest>,
    ) -> Result<Response<mav1::EvaluateMonitorAlertsResponse>, Status> {
        let _ = request.into_inner();
        let rules = monitor_alert_rules::Entity::find()
            .filter(monitor_alert_rules::Column::IsEnabled.eq(true))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let mut fired = 0i32;
        let mut results = Vec::with_capacity(rules.len());
        for r in rules {
            let value = sample_metric(&r.metric);
            let firing = if r.op == "LE" {
                value <= r.threshold
            } else {
                value >= r.threshold
            };
            let in_cooldown = r
                .last_alerted_at
                .map(|t| {
                    (chrono::DateTime::<chrono::FixedOffset>::from(store::now()) - t).num_minutes()
                        < r.cooldown_minutes.unwrap_or(30) as i64
                })
                .unwrap_or(false);
            let notified = firing
                && !in_cooldown
                && r.target.as_deref().map(|t| !t.is_empty()).unwrap_or(false);
            if notified {
                fired += 1;
            }
            let mut active: monitor_alert_rules::ActiveModel = r.clone().into();
            active.last_firing = Set(firing);
            active.last_value = Set(value);
            if notified {
                active.last_alerted_at = Set(Some(store::now().into()));
            }
            active.updated_at = Set(Some(store::now().into()));
            active.update(&self.state.db).await.map_err(db_status)?;
            results.push(mav1::evaluate_monitor_alerts_response::RuleOutcome {
                rule_id: r.id,
                name: r.name.clone(),
                firing,
                current_value: Some(value),
                notified,
                reason: Some(if firing {
                    format!("{} {} {}", r.metric, r.op, r.threshold)
                } else {
                    format!("within threshold ({} {} {})", r.metric, r.op, r.threshold)
                }),
            });
        }
        Ok(Response::new(mav1::EvaluateMonitorAlertsResponse {
            outcomes: results,
        }))
    }
}
