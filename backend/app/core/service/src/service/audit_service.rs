//! The audit services — the five write-ledger faces (the reference's
//! *_audit_log_service.go family): the API access trail, the login
//! trail, the operation trail (before/after), the data-access trail,
//! and the permission-change trail. Create writes the row verbatim
//! (the audit write is the point — the read faces map the core
//! columns back and carry the full payload in the entities); the
//! BFF's logging middleware feeds Create through the proxies.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{db_status, AppState};
use store::entities::{
    api_audit_logs, data_access_audit_logs, login_audit_logs, operation_audit_logs,
    permission_audit_logs,
};
use store::paging::fetch_paged;

use shop_proto::proto::audit::service::v1 as auditv1;
use shop_proto::proto::pagination::PagingRequest;

fn json_of(v: Option<String>) -> Option<serde_json::Value> {
    v.and_then(|s| serde_json::from_str(&s).ok())
}

fn s_of(v: Option<serde_json::Value>) -> Option<String> {
    v.map(|j| j.to_string())
}

// ── ApiAuditLogService ──────────────────────────────────────────────

pub struct ApiAuditLogService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl auditv1::api_audit_log_service_server::ApiAuditLogService for ApiAuditLogService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<auditv1::ListApiAuditLogResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            api_audit_logs::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        let items = rows
            .into_iter()
            .map(|r| auditv1::ApiAuditLog {
                id: Some(r.id),
                tenant_id: r.tenant_id,
                user_id: r.user_id,
                username: r.username.clone(),
                ip_address: r.ip_address.clone(),
                http_method: r.http_method.clone(),
                path: r.path.clone(),
                request_uri: r.request_uri.clone(),
                api_module: r.api_module.clone(),
                ..Default::default()
            })
            .collect();
        Ok(Response::new(auditv1::ListApiAuditLogResponse {
            items,
            total: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<auditv1::GetApiAuditLogRequest>,
    ) -> Result<Response<auditv1::ApiAuditLog>, Status> {
        let req = request.into_inner();
        let Some(auditv1::get_api_audit_log_request::QueryBy::Id(id)) = req.query_by else {
            return Err(Status::invalid_argument("query_by required"));
        };
        let row = api_audit_logs::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("api audit log not found"))?;
        Ok(Response::new(auditv1::ApiAuditLog {
            id: Some(row.id),
            tenant_id: row.tenant_id,
            user_id: row.user_id,
            username: row.username.clone(),
            ip_address: row.ip_address.clone(),
            http_method: row.http_method.clone(),
            path: row.path.clone(),
            request_uri: row.request_uri.clone(),
            api_module: row.api_module.clone(),
            ..Default::default()
        }))
    }

    async fn create(
        &self,
        request: Request<auditv1::CreateApiAuditLogRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req
            .data
            .ok_or_else(|| Status::invalid_argument("data required"))?;
        let active = api_audit_logs::ActiveModel {
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            user_id: Set(data.user_id),
            username: Set(data.username.clone()),
            ip_address: Set(data.ip_address.clone()),
            http_method: Set(data.http_method.clone()),
            path: Set(data.path.clone()),
            request_uri: Set(data.request_uri.clone()),
            api_module: Set(data.api_module.clone()),
            payload: Set(None),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

// ── LoginAuditLogService ────────────────────────────────────────────

pub struct LoginAuditLogService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl auditv1::login_audit_log_service_server::LoginAuditLogService for LoginAuditLogService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<auditv1::ListLoginAuditLogResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            login_audit_logs::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        let items = rows
            .into_iter()
            .map(|r| auditv1::LoginAuditLog {
                id: Some(r.id),
                tenant_id: r.tenant_id,
                user_id: r.user_id,
                username: r.username.clone(),
                ip_address: r.ip_address.clone(),
                session_id: r.session_id.clone(),
                request_id: r.request_id.clone(),
                trace_id: r.trace_id.clone(),
                failure_reason: r.failure_reason.clone(),
                ..Default::default()
            })
            .collect();
        Ok(Response::new(auditv1::ListLoginAuditLogResponse {
            items,
            total: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<auditv1::GetLoginAuditLogRequest>,
    ) -> Result<Response<auditv1::LoginAuditLog>, Status> {
        let req = request.into_inner();
        let Some(auditv1::get_login_audit_log_request::QueryBy::Id(id)) = req.query_by else {
            return Err(Status::invalid_argument("query_by required"));
        };
        let row = login_audit_logs::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("login audit log not found"))?;
        Ok(Response::new(auditv1::LoginAuditLog {
            id: Some(row.id),
            tenant_id: row.tenant_id,
            user_id: row.user_id,
            username: row.username.clone(),
            ip_address: row.ip_address.clone(),
            session_id: row.session_id.clone(),
            request_id: row.request_id.clone(),
            trace_id: row.trace_id.clone(),
            failure_reason: row.failure_reason.clone(),
            ..Default::default()
        }))
    }

    async fn create(
        &self,
        request: Request<auditv1::CreateLoginAuditLogRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req
            .data
            .ok_or_else(|| Status::invalid_argument("data required"))?;
        let active = login_audit_logs::ActiveModel {
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            user_id: Set(data.user_id),
            username: Set(data.username.clone()),
            ip_address: Set(data.ip_address.clone()),
            session_id: Set(data.session_id.clone()),
            request_id: Set(data.request_id.clone()),
            trace_id: Set(data.trace_id.clone()),
            failure_reason: Set(data.failure_reason.clone()),
            payload: Set(None),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

// ── OperationAuditLogService ────────────────────────────────────────

pub struct OperationAuditLogService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl auditv1::operation_audit_log_service_server::OperationAuditLogService
    for OperationAuditLogService
{
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<auditv1::ListOperationAuditLogResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            operation_audit_logs::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        let items = rows
            .into_iter()
            .map(|r| auditv1::OperationAuditLog {
                id: Some(r.id),
                tenant_id: r.tenant_id,
                user_id: r.user_id,
                username: r.username.clone(),
                resource_type: r.resource_type.clone(),
                resource_id: r.resource_id.clone(),
                before_data: r.before_data.clone().and_then(|j| j.to_string().into()),
                after_data: r.after_data.clone().and_then(|j| j.to_string().into()),
                request_id: r.request_id.clone(),
                ..Default::default()
            })
            .collect();
        Ok(Response::new(auditv1::ListOperationAuditLogResponse {
            items,
            total: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<auditv1::GetOperationAuditLogRequest>,
    ) -> Result<Response<auditv1::OperationAuditLog>, Status> {
        let req = request.into_inner();
        let Some(auditv1::get_operation_audit_log_request::QueryBy::Id(id)) = req.query_by else {
            return Err(Status::invalid_argument("query_by required"));
        };
        let row = operation_audit_logs::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("operation audit log not found"))?;
        Ok(Response::new(auditv1::OperationAuditLog {
            id: Some(row.id),
            tenant_id: row.tenant_id,
            user_id: row.user_id,
            username: row.username.clone(),
            resource_type: row.resource_type.clone(),
            resource_id: row.resource_id.clone(),
            before_data: row.before_data.clone().and_then(|j| j.to_string().into()),
            after_data: row.after_data.clone().and_then(|j| j.to_string().into()),
            request_id: row.request_id.clone(),
            ..Default::default()
        }))
    }

    async fn create(
        &self,
        request: Request<auditv1::CreateOperationAuditLogRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req
            .data
            .ok_or_else(|| Status::invalid_argument("data required"))?;
        let active = operation_audit_logs::ActiveModel {
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            user_id: Set(data.user_id),
            username: Set(data.username.clone()),
            resource_type: Set(data.resource_type.clone()),
            resource_id: Set(data.resource_id.clone()),
            before_data: Set(json_of(data.before_data.clone())),
            after_data: Set(json_of(data.after_data.clone())),
            request_id: Set(data.request_id.clone()),
            payload: Set(None),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

// ── DataAccessAuditLogService ───────────────────────────────────────

pub struct DataAccessAuditLogService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl auditv1::data_access_audit_log_service_server::DataAccessAuditLogService
    for DataAccessAuditLogService
{
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<auditv1::ListDataAccessAuditLogResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            data_access_audit_logs::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        let items = rows
            .into_iter()
            .map(|r| auditv1::DataAccessAuditLog {
                id: Some(r.id),
                tenant_id: r.tenant_id,
                user_id: r.user_id,
                username: r.username.clone(),
                ip_address: r.ip_address.clone(),
                request_id: r.request_id.clone(),
                data_source: r.data_source.clone(),
                table_name: r.table_name.clone(),
                data_id: r.data_id.clone(),
                sql_digest: r.sql_digest.clone(),
                ..Default::default()
            })
            .collect();
        Ok(Response::new(auditv1::ListDataAccessAuditLogResponse {
            items,
            total: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<auditv1::GetDataAccessAuditLogRequest>,
    ) -> Result<Response<auditv1::DataAccessAuditLog>, Status> {
        let req = request.into_inner();
        let Some(auditv1::get_data_access_audit_log_request::QueryBy::Id(id)) = req.query_by else {
            return Err(Status::invalid_argument("query_by required"));
        };
        let row = data_access_audit_logs::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("data access audit log not found"))?;
        Ok(Response::new(auditv1::DataAccessAuditLog {
            id: Some(row.id),
            tenant_id: row.tenant_id,
            user_id: row.user_id,
            username: row.username.clone(),
            ip_address: row.ip_address.clone(),
            request_id: row.request_id.clone(),
            data_source: row.data_source.clone(),
            table_name: row.table_name.clone(),
            data_id: row.data_id.clone(),
            sql_digest: row.sql_digest.clone(),
            ..Default::default()
        }))
    }

    async fn create(
        &self,
        request: Request<auditv1::CreateDataAccessAuditLogRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req
            .data
            .ok_or_else(|| Status::invalid_argument("data required"))?;
        let active = data_access_audit_logs::ActiveModel {
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            user_id: Set(data.user_id),
            username: Set(data.username.clone()),
            ip_address: Set(data.ip_address.clone()),
            request_id: Set(data.request_id.clone()),
            data_source: Set(data.data_source.clone()),
            table_name: Set(data.table_name.clone()),
            data_id: Set(data.data_id.clone()),
            sql_digest: Set(data.sql_digest.clone()),
            payload: Set(None),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

// ── PermissionAuditLogService ───────────────────────────────────────

pub struct PermissionAuditLogService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl auditv1::permission_audit_log_service_server::PermissionAuditLogService
    for PermissionAuditLogService
{
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<auditv1::ListPermissionAuditLogResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            permission_audit_logs::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        let items = rows
            .into_iter()
            .map(|r| auditv1::PermissionAuditLog {
                id: Some(r.id),
                tenant_id: r.tenant_id,
                operator_id: r.operator_id,
                operator_name: r.operator_name.clone(),
                target_type: r.target_type.clone(),
                target_id: r.target_id.clone(),
                target_name: r.target_name.clone(),
                old_value: r.old_value.clone().and_then(|j| j.to_string().into()),
                new_value: r.new_value.clone().and_then(|j| j.to_string().into()),
                ip_address: r.ip_address.clone(),
                request_id: r.request_id.clone(),
                ..Default::default()
            })
            .collect();
        Ok(Response::new(auditv1::ListPermissionAuditLogResponse {
            items,
            total: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<auditv1::GetPermissionAuditLogRequest>,
    ) -> Result<Response<auditv1::PermissionAuditLog>, Status> {
        let req = request.into_inner();
        let Some(auditv1::get_permission_audit_log_request::QueryBy::Id(id)) = req.query_by else {
            return Err(Status::invalid_argument("query_by required"));
        };
        let row = permission_audit_logs::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("permission audit log not found"))?;
        Ok(Response::new(auditv1::PermissionAuditLog {
            id: Some(row.id),
            tenant_id: row.tenant_id,
            operator_id: row.operator_id,
            operator_name: row.operator_name.clone(),
            target_type: row.target_type.clone(),
            target_id: row.target_id.clone(),
            target_name: row.target_name.clone(),
            old_value: row.old_value.clone().and_then(|j| j.to_string().into()),
            new_value: row.new_value.clone().and_then(|j| j.to_string().into()),
            ip_address: row.ip_address.clone(),
            request_id: row.request_id.clone(),
            ..Default::default()
        }))
    }

    async fn create(
        &self,
        request: Request<auditv1::CreatePermissionAuditLogRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req
            .data
            .ok_or_else(|| Status::invalid_argument("data required"))?;
        let active = permission_audit_logs::ActiveModel {
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            operator_id: Set(data.operator_id),
            operator_name: Set(data.operator_name.clone()),
            target_type: Set(data.target_type.clone()),
            target_id: Set(data.target_id.clone()),
            target_name: Set(data.target_name.clone()),
            old_value: Set(json_of(data.old_value.clone())),
            new_value: Set(json_of(data.new_value.clone())),
            ip_address: Set(data.ip_address.clone()),
            request_id: Set(data.request_id.clone()),
            payload: Set(None),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}
