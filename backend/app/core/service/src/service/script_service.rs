//! The script face (the reference's script_service.go over the
//! framework's script engine matrix) — the CRUD over `sys_scripts`,
//! the TestRun that compiles and executes the draft (or the stored
//! source) through the config-selected engine crate (LUA →
//! rushwind-script-lua, JAVASCRIPT → rushwind-script-javascript) with
//! the input map injected as the `input` global, and the log face
//! over `sys_script_logs`. ListHookPoints reads the mounted hook
//! points off the ledger — the hook catalog is what the scripts
//! declare, not what the code hard-codes.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_from_proto, ts_to_proto, AppState};
use store::entities::{sys_script_logs, sys_scripts};
use store::paging::fetch_paged;

use shop_proto::proto::pagination::PagingRequest;
use shop_proto::proto::script::service::v1 as scriptv1;

fn language_name(v: i32) -> Option<&'static str> {
    Some(match scriptv1::Language::try_from(v).ok()? {
        scriptv1::Language::Javascript => "JAVASCRIPT",
        _ => "LUA",
    })
}

/// The engine-crate registry name the language rides.
fn engine_type(language: Option<&str>) -> &'static str {
    match language {
        Some("JAVASCRIPT") => "javascript",
        _ => "lua",
    }
}

fn script_proto(r: sys_scripts::Model) -> scriptv1::Script {
    scriptv1::Script {
        id: Some(r.id),
        name: r.name,
        language: r.language.as_deref().map(|name| {
            if name == "JAVASCRIPT" {
                scriptv1::Language::Javascript as i32
            } else {
                scriptv1::Language::Lua as i32
            }
        }),
        hook_point: r.hook_point,
        source: r.source,
        priority: r.priority,
        description: r.description,
        critical: r.critical,
        version: r.version.map(|v| v as u32),
        is_enabled: r.is_enabled,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

async fn by_id(db: &sea_orm::DatabaseConnection, id: i64) -> Result<sys_scripts::Model, Status> {
    sys_scripts::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(db_status)?
        .ok_or_else(|| Status::not_found("script not found"))
}

pub struct ScriptService {
    pub state: Arc<AppState>,
}

impl ScriptService {
    /// The run half of TestRun: source through the language's engine
    /// with the input map injected as the `input` global; the return
    /// value (a map, or anything renderable) rides the context. The
    /// run is fenced inside a fresh engine instance per call — scripts
    /// share nothing between runs.
    async fn run_script(
        &self,
        language: Option<&str>,
        source: &str,
        input: &std::collections::HashMap<String, String>,
    ) -> (
        bool,
        Option<String>,
        std::collections::HashMap<String, String>,
        i64,
    ) {
        let started = std::time::Instant::now();
        let typ = engine_type(language);
        // The engine crates' self-registrations (idempotent - a taken
        // name just errors back).
        rushwind_script_lua::register();
        rushwind_script_javascript::register();
        let engine = match rushwind_script::new_script_engine(typ) {
            Ok(e) => e,
            Err(e) => {
                return (
                    false,
                    Some(format!("engine {typ}: {e:?}")),
                    Default::default(),
                    started.elapsed().as_millis() as i64,
                )
            }
        };
        if let Err(e) = engine.init().await {
            return (
                false,
                Some(format!("init: {e:?}")),
                Default::default(),
                started.elapsed().as_millis() as i64,
            );
        }
        // The data-only bridge: the input map rides in as the `input`
        // global, string-keyed, string-valued.
        let accessor = engine.clone().as_global_accessor();
        if let Some(accessor) = accessor.as_ref() {
            let globals: std::collections::HashMap<String, rushwind_script::ScriptValue> = input
                .iter()
                .map(|(k, v)| (k.clone(), rushwind_script::ScriptValue::String(v.clone())))
                .collect();
            let _ = accessor.register_global("input", rushwind_script::ScriptValue::Map(globals));
        }
        let outcome = match engine.clone().as_executor() {
            Some(executor) => executor.execute_string("test-run", source).await,
            None => Err(rushwind_script::ScriptError::Failed(format!(
                "engine {typ} carries no executor capability"
            ))),
        };
        let duration_ms = started.elapsed().as_millis() as i64;
        let mut context = std::collections::HashMap::new();
        match outcome {
            Ok(value) => {
                if let Some(accessor) = accessor.as_ref() {
                    if let Ok(result) = accessor.get_global("result") {
                        collect_string_map(&result, &mut context);
                    }
                }
                if context.is_empty() {
                    collect_string_map(&value, &mut context);
                }
                (true, None, context, duration_ms)
            }
            Err(e) => (false, Some(format!("{e:?}")), context, duration_ms),
        }
    }

    /// The log row a run leaves (best-effort — a logging failure never
    /// fails the run's caller).
    #[allow(clippy::too_many_arguments)]
    async fn record_run(
        &self,
        script_id: i64,
        name: &str,
        language: &str,
        hook_point: &str,
        version: i32,
        success: bool,
        error: Option<String>,
        context: &std::collections::HashMap<String, String>,
        duration_ms: i64,
    ) {
        let ctx = serde_json::to_value(context).unwrap_or(serde_json::json!({}));
        let _ = sys_script_logs::ActiveModel {
            script_id: Set(Some(script_id)),
            script_name: Set(Some(name.to_string())),
            language: Set(Some(language.to_string())),
            trigger_type: Set(Some("TEST_RUN".to_string())),
            hook_point: Set(Some(hook_point.to_string())),
            version: Set(Some(version)),
            success: Set(Some(success)),
            duration_ms: Set(Some(duration_ms)),
            error: Set(error),
            context: Set(Some(ctx)),
            created_at: Set(Some(store::now())),
            ..Default::default()
        }
        .insert(&self.state.db)
        .await;
    }
}

fn collect_string_map(
    value: &rushwind_script::ScriptValue,
    out: &mut std::collections::HashMap<String, String>,
) {
    if let rushwind_script::ScriptValue::Map(map) = value {
        for (k, v) in map {
            out.insert(k.clone(), render_value(v));
        }
    }
}

fn render_value(v: &rushwind_script::ScriptValue) -> String {
    match v {
        rushwind_script::ScriptValue::Null => String::new(),
        rushwind_script::ScriptValue::Bool(b) => b.to_string(),
        rushwind_script::ScriptValue::Int(i) => i.to_string(),
        rushwind_script::ScriptValue::UInt(u) => u.to_string(),
        rushwind_script::ScriptValue::Float(f) => f.to_string(),
        rushwind_script::ScriptValue::String(s) => s.clone(),
        other => format!("{other:?}"),
    }
}

#[async_trait::async_trait]
impl scriptv1::script_service_server::ScriptService for ScriptService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<scriptv1::ListScriptsResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sys_scripts::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(scriptv1::ListScriptsResponse {
            items: rows.into_iter().map(script_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<scriptv1::CountScriptsResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            sys_scripts::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(scriptv1::CountScriptsResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<scriptv1::GetScriptRequest>,
    ) -> Result<Response<scriptv1::Script>, Status> {
        let req = request.into_inner();
        let row = match req.query_by {
            Some(scriptv1::get_script_request::QueryBy::Id(id)) => {
                by_id(&self.state.db, id).await?
            }
            Some(scriptv1::get_script_request::QueryBy::Name(name)) => sys_scripts::Entity::find()
                .filter(sys_scripts::Column::Name.eq(name))
                .one(&self.state.db)
                .await
                .map_err(db_status)?
                .ok_or_else(|| Status::not_found("script not found"))?,
            None => return Err(bad("query_by required")),
        };
        Ok(Response::new(script_proto(row)))
    }

    async fn create(
        &self,
        request: Request<scriptv1::CreateScriptRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let active = sys_scripts::ActiveModel {
            name: Set(data.name),
            language: Set(data.language.and_then(language_name).map(String::from)),
            hook_point: Set(data.hook_point),
            source: Set(data.source),
            priority: Set(data.priority),
            description: Set(data.description),
            critical: Set(data.critical),
            version: Set(Some(1)),
            is_enabled: Set(data.is_enabled),
            created_by: Set(data.created_by),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<scriptv1::UpdateScriptRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let existing = by_id(&self.state.db, req.id).await?;
        let mut active: sys_scripts::ActiveModel = existing.into();
        if let Some(v) = data.name {
            active.name = Set(Some(v));
        }
        if let Some(v) = data.language {
            active.language = Set(language_name(v).map(String::from));
        }
        if let Some(v) = data.hook_point {
            active.hook_point = Set(Some(v));
        }
        if let Some(v) = data.source {
            active.source = Set(Some(v));
            // A source change bumps the version (the reference's
            // monotonic revision).
            active.version = Set(Some(active.version.clone().unwrap().unwrap_or(1) + 1));
        }
        if let Some(v) = data.priority {
            active.priority = Set(Some(v));
        }
        if let Some(v) = data.description {
            active.description = Set(Some(v));
        }
        if let Some(v) = data.critical {
            active.critical = Set(Some(v));
        }
        if let Some(v) = data.is_enabled {
            active.is_enabled = Set(Some(v));
        }
        if let Some(v) = data.updated_by {
            active.updated_by = Set(Some(v));
        }
        active.updated_at = Set(Some(store::now()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<scriptv1::DeleteScriptRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        if req.ids.is_empty() {
            return Err(bad("ids required"));
        }
        for id in &req.ids {
            sys_script_logs::Entity::delete_many()
                .filter(sys_script_logs::Column::ScriptId.eq(*id))
                .exec(&self.state.db)
                .await
                .map_err(db_status)?;
        }
        sys_scripts::Entity::delete_many()
            .filter(sys_scripts::Column::Id.is_in(req.ids))
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The sandbox run: draft or stored source, the input map in, the
    /// result map out, the log row behind.
    async fn test_run(
        &self,
        request: Request<scriptv1::TestRunScriptRequest>,
    ) -> Result<Response<scriptv1::TestRunScriptResponse>, Status> {
        let req = request.into_inner();
        let (script_id, name, language, hook_point, version, source) = match req.target {
            Some(scriptv1::test_run_script_request::Target::Id(id)) => {
                let row = by_id(&self.state.db, id).await?;
                (
                    row.id,
                    row.name.unwrap_or_default(),
                    row.language.clone(),
                    row.hook_point.unwrap_or_default(),
                    row.version.unwrap_or(1),
                    row.source.unwrap_or_default(),
                )
            }
            Some(scriptv1::test_run_script_request::Target::Draft(draft)) => (
                0,
                draft.name.unwrap_or_else(|| "<draft>".to_string()),
                draft.language.and_then(language_name).map(String::from),
                draft.hook_point.unwrap_or_default(),
                draft.version.unwrap_or(0) as i32,
                draft.source.unwrap_or_default(),
            ),
            None => return Err(bad("target required")),
        };
        let language_spelling = language.as_deref().unwrap_or("LUA").to_string();
        let (success, error, context, duration_ms) = self
            .run_script(language.as_deref(), &source, &req.input)
            .await;
        self.record_run(
            script_id,
            &name,
            &language_spelling,
            &hook_point,
            version,
            success,
            error.clone(),
            &context,
            duration_ms,
        )
        .await;
        Ok(Response::new(scriptv1::TestRunScriptResponse {
            success,
            error: error.unwrap_or_default(),
            context,
            duration_ms,
        }))
    }

    /// The mounted hook points off the ledger — the names the stored
    /// scripts declare, each with its mount count; the languages list
    /// is the engine matrix this build carries.
    async fn list_hook_points(
        &self,
        _request: Request<pbjson_types::Empty>,
    ) -> Result<Response<scriptv1::ListHookPointsResponse>, Status> {
        let rows = sys_scripts::Entity::find()
            .filter(sys_scripts::Column::HookPoint.is_not_null())
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let mut counts: std::collections::BTreeMap<String, i64> = Default::default();
        for r in rows {
            if let Some(hp) = r.hook_point.filter(|s| !s.is_empty()) {
                *counts.entry(hp).or_insert(0) += 1;
            }
        }
        let items = counts
            .into_iter()
            .map(|(name, count)| scriptv1::HookPoint {
                name,
                description: String::new(),
                script_count: count as u32,
            })
            .collect();
        Ok(Response::new(scriptv1::ListHookPointsResponse {
            items,
            languages: rushwind_script::list_factories(),
        }))
    }
}

// ── ScriptLog ────────────────────────────────────────────────────────

fn log_proto(r: sys_script_logs::Model) -> scriptv1::ScriptLog {
    scriptv1::ScriptLog {
        id: Some(r.id),
        script_id: r.script_id,
        script_name: r.script_name,
        language: r.language,
        trigger_type: r.trigger_type,
        hook_point: r.hook_point,
        version: r.version.map(|v| v as u32),
        success: r.success,
        duration_ms: r.duration_ms,
        error: r.error,
        created_at: r.created_at.and_then(ts_to_proto),
    }
}

pub struct ScriptLogService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl scriptv1::script_log_service_server::ScriptLogService for ScriptLogService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<scriptv1::ListScriptLogsResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sys_script_logs::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(scriptv1::ListScriptLogsResponse {
            items: rows.into_iter().map(log_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<scriptv1::CountScriptLogsResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            sys_script_logs::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(scriptv1::CountScriptLogsResponse {
            count: total as i64,
        }))
    }

    async fn purge(
        &self,
        request: Request<scriptv1::PurgeScriptLogsRequest>,
    ) -> Result<Response<scriptv1::PurgeScriptLogsResponse>, Status> {
        let req = request.into_inner();
        let mut delete = sys_script_logs::Entity::delete_many();
        if let Some(before) = req.before {
            let cutoff: chrono::DateTime<chrono::FixedOffset> = ts_from_proto(before);
            delete = delete.filter(sys_script_logs::Column::CreatedAt.lt(cutoff));
        }
        let res = delete.exec(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(scriptv1::PurgeScriptLogsResponse {
            deleted: res.rows_affected,
        }))
    }
}
