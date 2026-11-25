//! The agent-app management face (the reference's ai_agent_service.go)
//! — the CRUD over the agent app definitions (the graph JSON) and the
//! run ledger. Run executes the graph as a LINEAR walk of its `nodes`
//! array: each node becomes one LLM call (system = the app graph's
//! description, user = the running input + the accumulated context),
//! node outputs append to the trace; an `approval`-typed node pauses
//! the walk at WAITING_APPROVAL (pending_node_id + resume_state carry
//! the remaining nodes), and ApproveRun resumes the stored remainder —
//! rejected approvals close the run as FAILED with the note. Failures
//! at any step land the run in FAILED with the provider error.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{sys_ai_agent_apps, sys_ai_agent_runs};
use store::paging::fetch_paged;

use shop_proto::proto::ai::service::v1 as aiv1;
use shop_proto::proto::pagination::PagingRequest;

fn app_proto(r: sys_ai_agent_apps::Model) -> aiv1::AiAgentApp {
    aiv1::AiAgentApp {
        id: Some(r.id),
        name: r.name,
        description: r.description,
        graph: r.graph,
        enabled: r.enabled,
        created_by: r.created_by,
        updated_by: r.updated_by,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

fn run_proto(r: sys_ai_agent_runs::Model) -> aiv1::AiAgentRun {
    let status = Some(
        aiv1::ai_agent_run::Status::from_str_name(r.status.as_deref().unwrap_or("RUNNING"))
            .map(|e| e as i32)
            .unwrap_or(aiv1::ai_agent_run::Status::Running as i32),
    );
    aiv1::AiAgentRun {
        id: Some(r.id),
        app_id: r.app_id,
        status,
        input: r.input,
        output: r.output,
        error: r.error,
        trace: r.trace,
        prompt_tokens: r.prompt_tokens.map(|v| v as u32),
        completion_tokens: r.completion_tokens.map(|v| v as u32),
        total_tokens: r.total_tokens.map(|v| v as u32),
        duration_ms: r.duration_ms.map(|v| v as u32),
        created_at: r.created_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

async fn app_by_id(
    db: &sea_orm::DatabaseConnection,
    id: i64,
) -> Result<sys_ai_agent_apps::Model, Status> {
    sys_ai_agent_apps::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(db_status)?
        .ok_or_else(|| Status::not_found("agent app not found"))
}

pub struct AiAgentService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl aiv1::ai_agent_service_server::AiAgentService for AiAgentService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<aiv1::ListAiAgentAppsResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sys_ai_agent_apps::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(aiv1::ListAiAgentAppsResponse {
            items: rows.into_iter().map(app_proto).collect(),
            total: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<aiv1::GetAiAgentAppRequest>,
    ) -> Result<Response<aiv1::AiAgentApp>, Status> {
        let req = request.into_inner();
        Ok(Response::new(app_proto(
            app_by_id(&self.state.db, req.id).await?,
        )))
    }

    async fn create(
        &self,
        request: Request<aiv1::CreateAiAgentAppRequest>,
    ) -> Result<Response<aiv1::AiAgentApp>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let active = sys_ai_agent_apps::ActiveModel {
            tenant_id: Set(Some(0)),
            name: Set(data.name),
            description: Set(data.description),
            graph: Set(data.graph),
            enabled: Set(data.enabled),
            created_by: Set(data.created_by),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        let row = active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(app_proto(row)))
    }

    async fn update(
        &self,
        request: Request<aiv1::UpdateAiAgentAppRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let existing = app_by_id(&self.state.db, req.id).await?;
        let mut active: sys_ai_agent_apps::ActiveModel = existing.into();
        if let Some(v) = data.name {
            active.name = Set(Some(v));
        }
        if let Some(v) = data.description {
            active.description = Set(Some(v));
        }
        if let Some(v) = data.graph {
            active.graph = Set(Some(v));
        }
        if let Some(v) = data.enabled {
            active.enabled = Set(Some(v));
        }
        if let Some(v) = data.updated_by {
            active.updated_by = Set(Some(v));
        }
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The cascade: the app's run ledger goes with it.
    async fn delete(
        &self,
        request: Request<aiv1::DeleteAiAgentAppRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        sys_ai_agent_runs::Entity::delete_many()
            .filter(sys_ai_agent_runs::Column::AppId.eq(req.id))
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        sys_ai_agent_apps::Entity::delete_by_id(req.id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The execution face: a linear walk of the graph's nodes, one LLM
    /// call per node, approval nodes pausing the walk.
    async fn run(
        &self,
        request: Request<aiv1::RunAiAgentAppRequest>,
    ) -> Result<Response<aiv1::AiAgentRun>, Status> {
        let req = request.into_inner();
        let app = app_by_id(&self.state.db, req.id).await?;
        if app.enabled != Some(true) {
            return Err(bad("agent app is disabled"));
        }
        let started = std::time::Instant::now();
        let run = sys_ai_agent_runs::ActiveModel {
            tenant_id: Set(Some(0)),
            app_id: Set(Some(app.id)),
            status: Set(Some("RUNNING".to_string())),
            input: Set(Some(serde_json::to_string(&req.input).unwrap_or_default())),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        }
        .insert(&self.state.db)
        .await
        .map_err(db_status)?;
        let run_id = run.id;

        let nodes = graph_nodes(app.graph.as_deref());
        let input_text = serde_json::to_string(&req.input).unwrap_or_default();
        match self
            .walk(&nodes, 0, String::new(), &input_text, &app, run_id, started)
            .await
        {
            Ok((output, pt, ct, trace)) => {
                let row = finish_run(
                    &self.state.db,
                    run_id,
                    "SUCCEEDED",
                    None,
                    output,
                    pt,
                    ct,
                    Some(trace),
                    started,
                )
                .await?;
                Ok(Response::new(run_proto(row)))
            }
            Err(WalkOutcome::Done(output, pt, ct, trace)) => {
                let row = finish_run(
                    &self.state.db,
                    run_id,
                    "SUCCEEDED",
                    None,
                    output,
                    pt,
                    ct,
                    Some(trace),
                    started,
                )
                .await?;
                Ok(Response::new(run_proto(row)))
            }
            Err(WalkOutcome::Paused {
                pending_node,
                resume_state,
                trace,
            }) => {
                let row =
                    pause_run(&self.state.db, run_id, &pending_node, &resume_state, &trace).await?;
                Ok(Response::new(run_proto(row)))
            }
            Err(WalkOutcome::Failed(e)) => {
                let row = finish_run(
                    &self.state.db,
                    run_id,
                    "FAILED",
                    Some(e.clone()),
                    String::new(),
                    0,
                    0,
                    None,
                    started,
                )
                .await?;
                Ok(Response::new(run_proto(row)))
            }
        }
    }

    /// The approval resume: WAITING_APPROVAL runs continue the stored
    /// remainder on approval; a rejection closes the run FAILED with
    /// the reviewer's note.
    async fn approve_run(
        &self,
        request: Request<aiv1::ApproveAgentRunRequest>,
    ) -> Result<Response<aiv1::AiAgentRun>, Status> {
        let req = request.into_inner();
        let row = sys_ai_agent_runs::Entity::find_by_id(req.run_id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("agent run not found"))?;
        if row.status.as_deref() != Some("WAITING_APPROVAL") {
            return Err(bad("run is not waiting for approval"));
        }
        let started = std::time::Instant::now();
        if !req.approved {
            let updated = finish_run(
                &self.state.db,
                row.id,
                "FAILED",
                Some(format!(
                    "rejected at node {}: {}",
                    row.pending_node_id.clone().unwrap_or_default(),
                    req.note
                )),
                String::new(),
                0,
                0,
                row.trace.clone(),
                started,
            )
            .await?;
            return Ok(Response::new(run_proto(updated)));
        }
        // Resume: the stored remainder rides resume_state.
        let resume: serde_json::Value =
            serde_json::from_str(row.resume_state.as_deref().unwrap_or("null"))
                .unwrap_or(serde_json::Value::Null);
        let nodes = resume
            .get("remaining")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let context = resume
            .get("context")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let trace_in = row.trace.clone().unwrap_or_default();
        let app = sys_ai_agent_apps::Entity::find_by_id(row.app_id.unwrap_or(0))
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("agent app not found"))?;
        match self
            .walk(
                &nodes,
                0,
                context,
                &row.input.clone().unwrap_or_default(),
                &app,
                row.id,
                started,
            )
            .await
        {
            Ok((output, pt, ct, trace)) => {
                let updated = finish_run(
                    &self.state.db,
                    row.id,
                    "SUCCEEDED",
                    None,
                    output,
                    pt,
                    ct,
                    Some(trace),
                    started,
                )
                .await?;
                Ok(Response::new(run_proto(updated)))
            }
            Err(WalkOutcome::Done(output, pt, ct, trace)) => {
                let updated = finish_run(
                    &self.state.db,
                    row.id,
                    "SUCCEEDED",
                    None,
                    output,
                    pt,
                    ct,
                    Some(trace),
                    started,
                )
                .await?;
                Ok(Response::new(run_proto(updated)))
            }
            Err(WalkOutcome::Paused {
                pending_node,
                resume_state,
                trace,
            }) => {
                let merged_trace = if trace_in.is_empty() {
                    trace
                } else {
                    format!(
                        "{trace_in}
{trace}"
                    )
                };
                let updated = pause_run(
                    &self.state.db,
                    row.id,
                    &pending_node,
                    &resume_state,
                    &merged_trace,
                )
                .await?;
                Ok(Response::new(run_proto(updated)))
            }
            Err(WalkOutcome::Failed(e)) => {
                let updated = finish_run(
                    &self.state.db,
                    row.id,
                    "FAILED",
                    Some(e),
                    String::new(),
                    0,
                    0,
                    None,
                    started,
                )
                .await?;
                Ok(Response::new(run_proto(updated)))
            }
        }
    }

    async fn list_runs(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<aiv1::ListAiAgentRunsResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sys_ai_agent_runs::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(aiv1::ListAiAgentRunsResponse {
            items: rows.into_iter().map(run_proto).collect(),
            total: total as i64,
        }))
    }

    async fn get_run(
        &self,
        request: Request<aiv1::GetAiAgentRunRequest>,
    ) -> Result<Response<aiv1::AiAgentRun>, Status> {
        let req = request.into_inner();
        let row = sys_ai_agent_runs::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("agent run not found"))?;
        Ok(Response::new(run_proto(row)))
    }
}

// ---------------------------------------------------------------------------
// The graph walk: nodes → LLM calls, approval nodes pause the walk.
// ---------------------------------------------------------------------------

/// The walk's outcomes: done with the accumulated output, paused at an
/// approval node, or failed with the provider error.
enum WalkOutcome {
    Done(String, i64, i64, String),
    Paused {
        pending_node: String,
        resume_state: String,
        trace: String,
    },
    Failed(String),
}

/// Extracts the graph's node list — an object with a `nodes` array, or
/// a bare array; anything else compiles to a single unnamed node so the
/// walk still runs one LLM call with the whole graph as context.
fn graph_nodes(graph: Option<&str>) -> Vec<serde_json::Value> {
    let parsed: serde_json::Value = serde_json::from_str(graph.unwrap_or("")).unwrap_or_default();
    if let Some(arr) = parsed.get("nodes").and_then(|v| v.as_array()) {
        return arr.clone();
    }
    if let Some(arr) = parsed.as_array() {
        return arr.clone();
    }
    vec![serde_json::json!({"type": "graph", "raw": parsed})]
}

fn node_label(node: &serde_json::Value) -> String {
    node.get("name")
        .or_else(|| node.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("node")
        .to_string()
}

fn node_is_approval(node: &serde_json::Value) -> bool {
    let t = node
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_lowercase();
    t.contains("approval") || t.contains("review")
}

fn node_prompt(node: &serde_json::Value, input: &str, context: &str) -> String {
    let desc = node
        .get("description")
        .or_else(|| node.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("执行这一步");
    format!(
        "Agent 图节点「{desc}」。
用户输入：{input}
前文上下文：{context}
请执行该节点的任务并直接输出结果。"
    )
}

impl crate::service::ai_agent_service::AiAgentService {
    /// Walks `nodes` from `start`, one LLM call per node, accumulating
    /// the context; an approval node pauses. Returns the final output
    /// (the last node's answer) plus token counts for the ledger.
    async fn walk(
        &self,
        nodes: &[serde_json::Value],
        start: usize,
        mut context: String,
        input: &str,
        app: &sys_ai_agent_apps::Model,
        run_id: i64,
        started: std::time::Instant,
    ) -> Result<(String, i64, i64, String), WalkOutcome> {
        let arr: &[serde_json::Value] = nodes;
        if arr.is_empty() {
            return Ok((context, 0, 0, String::new()));
        }
        let provider = crate::service::ai_runtime_service::resolve_provider(&self.state, None)
            .await
            .map_err(|s| WalkOutcome::Failed(s.message().to_string()))?;
        let system = format!(
            "你是 agent「{}」的执行引擎。按节点指令逐步完成任务，直接输出结果。",
            app.name.clone().unwrap_or_else(|| "agent".to_string())
        );
        let mut trace = String::new();
        let mut pt_total: i64 = 0;
        let mut ct_total: i64 = 0;
        for (idx, node) in arr.iter().enumerate().skip(start) {
            let label = node_label(node);
            if node_is_approval(node) {
                // Pause: the remainder + the context ride resume_state.
                let remaining: Vec<serde_json::Value> = arr[(idx + 1)..].to_vec();
                let resume = serde_json::json!({
                    "remaining": remaining,
                    "context": context,
                });
                trace.push_str(&format!(
                    "[paused at {label} (approval gate)]
"
                ));
                return Err(WalkOutcome::Paused {
                    pending_node: label,
                    resume_state: resume.to_string(),
                    trace,
                });
            }
            let prompt = node_prompt(node, input, &context);
            let started_node = std::time::Instant::now();
            let (answer, pt, ct, model) = crate::service::ai_runtime_service::chat_completion(
                &provider,
                Some(&system),
                &prompt,
                0,
            )
            .await
            .map_err(|s| WalkOutcome::Failed(s.message().to_string()))?;
            let node_ms = started_node.elapsed().as_millis() as i64;
            pt_total += pt;
            ct_total += ct;
            trace.push_str(&format!(
                "[{idx} {label} | {model} | {node_ms}ms]
{answer}
"
            ));
            if context.len() > 8000 {
                context = format!("…{}…", &context[context.len() - 4000..]);
            }
            context.push_str(
                "
",
            );
            context.push_str(&answer);
        }
        let _ = run_id;
        let output = context.trim().to_string();
        Ok((output, pt_total, ct_total, trace))
    }
}

/// Moves a finished/paused run's ledger row (best-effort trace merge —
/// the caller composes the final trace text).
async fn pause_run(
    db: &sea_orm::DatabaseConnection,
    run_id: i64,
    pending_node: &str,
    resume_state: &str,
    trace: &str,
) -> Result<sys_ai_agent_runs::Model, Status> {
    let row = sys_ai_agent_runs::Entity::find_by_id(run_id)
        .one(db)
        .await
        .map_err(db_status)?
        .ok_or_else(|| Status::not_found("agent run not found"))?;
    let mut active: sys_ai_agent_runs::ActiveModel = row.into();
    active.status = Set(Some("WAITING_APPROVAL".to_string()));
    active.pending_node_id = Set(Some(pending_node.to_string()));
    active.resume_state = Set(Some(resume_state.to_string()));
    active.trace = Set(Some(trace.to_string()));
    active.updated_at = Set(Some(store::now().into()));
    active.update(db).await.map_err(db_status)
}

/// Lands the terminal state with the tokens/duration snapshot.
#[allow(clippy::too_many_arguments)]
async fn finish_run(
    db: &sea_orm::DatabaseConnection,
    run_id: i64,
    status: &str,
    error: Option<String>,
    output: String,
    prompt_tokens: i64,
    completion_tokens: i64,
    trace: Option<String>,
    started: std::time::Instant,
) -> Result<sys_ai_agent_runs::Model, Status> {
    let row = sys_ai_agent_runs::Entity::find_by_id(run_id)
        .one(db)
        .await
        .map_err(db_status)?
        .ok_or_else(|| Status::not_found("agent run not found"))?;
    let mut active: sys_ai_agent_runs::ActiveModel = row.into();
    active.status = Set(Some(status.to_string()));
    active.output = Set(if output.is_empty() {
        None
    } else {
        Some(output)
    });
    active.error = Set(error);
    active.prompt_tokens = Set(Some(prompt_tokens));
    active.completion_tokens = Set(Some(completion_tokens));
    active.total_tokens = Set(Some(prompt_tokens + completion_tokens));
    active.duration_ms = Set(Some(started.elapsed().as_millis() as i64));
    if let Some(t) = trace.filter(|t| !t.is_empty()) {
        active.trace = Set(Some(t));
    }
    active.updated_at = Set(Some(store::now().into()));
    active.update(db).await.map_err(db_status)
}
