//! The agent-app management face (the reference's ai_agent_service.go)
//! — the CRUD over the agent app definitions (the graph JSON) and the
//! run ledger. The Run/ApproveRun execution faces answer
//! Unimplemented this round: executing a graph needs the LLM runtime
//! the providers configure, the face lands with the AI runtime (the
//! same known gap the chat/content/query faces carry).

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

    /// The execution face — the graph runs through the LLM runtime the
    /// provider rows configure; the runtime lands with the AI runtime
    /// batch (the same gap Chat/Generate carry).
    async fn run(
        &self,
        _request: Request<aiv1::RunAiAgentAppRequest>,
    ) -> Result<Response<aiv1::AiAgentRun>, Status> {
        Err(Status::unimplemented("agent runtime not yet implemented"))
    }

    async fn approve_run(
        &self,
        _request: Request<aiv1::ApproveAgentRunRequest>,
    ) -> Result<Response<aiv1::AiAgentRun>, Status> {
        Err(Status::unimplemented("agent runtime not yet implemented"))
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
