//! The task service — the scheduled-task configuration face (the
//! reference's task_service.go). The CRUD walks the sys_tasks config
//! table; the runtime-control faces (StartAll/StopAll/RestartAll/
//! ControlTask) ride the apalis worker's management bridge and stay on
//! their generated stubs until that pipeline lands.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, AppState};
use store::entities::tasks;
use store::paging::fetch_paged;

use shop_proto::proto::pagination::PagingRequest;
use shop_proto::proto::task::service::v1 as taskv1;

fn task_proto(r: tasks::Model) -> taskv1::Task {
    let task_type = if r.r#type == "DELAY" {
        taskv1::task::Type::Delay
    } else if r.r#type == "WAIT_RESULT" {
        taskv1::task::Type::WaitResult
    } else {
        taskv1::task::Type::Periodic
    };
    taskv1::Task {
        id: Some(r.id),
        r#type: Some(task_type as i32),
        type_name: Some(r.type_name.clone()),
        task_payload: r.payload.clone(),
        cron_spec: r.cron_spec.clone(),
        enable: r.enable,
        remark: r.remark.clone(),
        tenant_id: Some(r.tenant_id.unwrap_or(0)),
        created_at: r.created_at.and_then(crate::state::ts_to_proto),
        updated_at: r.updated_at.and_then(crate::state::ts_to_proto),
        ..Default::default()
    }
}

/// The task type names this deployment registers (the handlers the
/// worker pipeline carries).
const REGISTERED_TYPES: &[&str] = &[
    "order_timeout_scan",
    "notification_email_deliver",
    "notification_webhook_deliver",
];

pub struct TaskService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl taskv1::task_service_server::TaskService for TaskService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<taskv1::ListTaskResponse>, Status> {
        let (rows, total) =
            fetch_paged(&self.state.db, tasks::Entity::find(), &request.into_inner())
                .await
                .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(taskv1::ListTaskResponse {
            items: rows.into_iter().map(task_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<taskv1::CountTaskResponse>, Status> {
        let (_, total) = fetch_paged(&self.state.db, tasks::Entity::find(), &request.into_inner())
            .await
            .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(taskv1::CountTaskResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<taskv1::GetTaskRequest>,
    ) -> Result<Response<taskv1::Task>, Status> {
        let req = request.into_inner();
        let Some(taskv1::get_task_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = tasks::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("task not found"))?;
        Ok(Response::new(task_proto(row)))
    }

    async fn create(
        &self,
        request: Request<taskv1::CreateTaskRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let type_name = data
            .type_name
            .clone()
            .filter(|t| !t.is_empty())
            .ok_or_else(|| bad("data.typeName required"))?;
        if !REGISTERED_TYPES.contains(&type_name.as_str()) {
            return Err(bad("unknown task type name"));
        }
        let active = tasks::ActiveModel {
            r#type: Set(
                match taskv1::task::Type::try_from(data.r#type.unwrap_or(0)).ok() {
                    Some(taskv1::task::Type::Delay) => "DELAY",
                    Some(taskv1::task::Type::WaitResult) => "WAIT_RESULT",
                    _ => "PERIODIC",
                }
                .to_string(),
            ),
            type_name: Set(type_name),
            payload: Set(data.task_payload.clone()),
            cron_spec: Set(data.cron_spec.clone()),
            options: Set(data
                .task_options
                .map(|o| serde_json::json!({ "max_retry": o.max_retry }))),
            enable: Set(Some(data.enable.unwrap_or(true))),
            remark: Set(data.remark.clone()),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<taskv1::UpdateTaskRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = tasks::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("task not found"))?;
        let mut active: tasks::ActiveModel = existing.into();
        if let Some(v) = data.task_payload.clone() {
            active.payload = Set(Some(v));
        }
        if let Some(v) = data.cron_spec.clone() {
            active.cron_spec = Set(Some(v));
        }
        if let Some(v) = data.enable {
            active.enable = Set(Some(v));
        }
        if let Some(v) = data.remark.clone() {
            active.remark = Set(Some(v));
        }
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<taskv1::DeleteTaskRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(taskv1::delete_task_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        tasks::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn list_task_type_name(
        &self,
        _request: Request<pbjson_types::Empty>,
    ) -> Result<Response<taskv1::ListTaskTypeNameResponse>, Status> {
        Ok(Response::new(taskv1::ListTaskTypeNameResponse {
            type_names: REGISTERED_TYPES.iter().map(|s| s.to_string()).collect(),
        }))
    }
}
