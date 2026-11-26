//! The plan family (the reference's plan_service.go) — the tenant
//! subscription plans with the expiry policy, the per-plan quotas, and
//! the per-plan module whitelist. Three same-shaped CRUD faces; the
//! varchar enum names the golden schema stores (`FREE` / `READONLY` /
//! `USER_LIMIT` / `DASHBOARD` …) map onto the proto enums at the face.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{sys_plan_modules, sys_plan_quotas, sys_plans};
use store::paging::fetch_paged;

use shop_proto::proto::identity::service::v1 as identityv1;
use shop_proto::proto::pagination::PagingRequest;

pub struct PlanService {
    pub state: Arc<AppState>,
}

fn version_name(v: i32) -> Option<&'static str> {
    Some(match identityv1::plan::Version::try_from(v).ok()? {
        identityv1::plan::Version::Standard => "STANDARD",
        identityv1::plan::Version::Enterprise => "ENTERPRISE",
        _ => "FREE",
    })
}

fn version_proto(name: Option<&str>) -> Option<i32> {
    Some(match name? {
        "STANDARD" => identityv1::plan::Version::Standard as i32,
        "ENTERPRISE" => identityv1::plan::Version::Enterprise as i32,
        _ => identityv1::plan::Version::Free as i32,
    })
}

fn expiry_name(v: i32) -> Option<&'static str> {
    Some(match identityv1::plan::ExpiryPolicy::try_from(v).ok()? {
        identityv1::plan::ExpiryPolicy::BlockLogin => "BLOCK_LOGIN",
        identityv1::plan::ExpiryPolicy::Freeze => "FREEZE",
        _ => "READONLY",
    })
}

fn expiry_proto(name: Option<&str>) -> Option<i32> {
    Some(match name? {
        "BLOCK_LOGIN" => identityv1::plan::ExpiryPolicy::BlockLogin as i32,
        "FREEZE" => identityv1::plan::ExpiryPolicy::Freeze as i32,
        _ => identityv1::plan::ExpiryPolicy::Readonly as i32,
    })
}

fn plan_proto(r: sys_plans::Model) -> identityv1::Plan {
    identityv1::Plan {
        id: Some(r.id),
        name: r.name,
        version: version_proto(r.version.as_deref()),
        expiry_policy: expiry_proto(r.expiry_policy.as_deref()),
        data_retention_days: r.data_retention_days.map(|v| v as u32),
        description: r.description,
        remark: r.remark,
        created_by: r.created_by,
        updated_by: r.updated_by,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

async fn by_id(db: &sea_orm::DatabaseConnection, id: i64) -> Result<sys_plans::Model, Status> {
    sys_plans::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(db_status)?
        .ok_or_else(|| Status::not_found("plan not found"))
}

#[async_trait::async_trait]
impl identityv1::plan_service_server::PlanService for PlanService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<identityv1::ListPlanResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sys_plans::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(identityv1::ListPlanResponse {
            items: rows.into_iter().map(plan_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<identityv1::CountPlanResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            sys_plans::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(identityv1::CountPlanResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<identityv1::GetPlanRequest>,
    ) -> Result<Response<identityv1::Plan>, Status> {
        let req = request.into_inner();
        let Some(identityv1::get_plan_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        Ok(Response::new(plan_proto(by_id(&self.state.db, id).await?)))
    }

    async fn create(
        &self,
        request: Request<identityv1::CreatePlanRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let active = sys_plans::ActiveModel {
            name: Set(data.name),
            version: Set(data.version.and_then(version_name).map(String::from)),
            expiry_policy: Set(data.expiry_policy.and_then(expiry_name).map(String::from)),
            data_retention_days: Set(data.data_retention_days.map(|v| v as i64)),
            description: Set(data.description),
            remark: Set(data.remark),
            created_by: Set(data.created_by),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<identityv1::UpdatePlanRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let existing = by_id(&self.state.db, req.id).await?;
        let mut active: sys_plans::ActiveModel = existing.into();
        if let Some(v) = data.name {
            active.name = Set(Some(v));
        }
        if let Some(v) = data.version {
            active.version = Set(version_name(v).map(String::from));
        }
        if let Some(v) = data.expiry_policy {
            active.expiry_policy = Set(expiry_name(v).map(String::from));
        }
        if let Some(v) = data.data_retention_days {
            active.data_retention_days = Set(Some(v as i64));
        }
        if let Some(v) = data.description {
            active.description = Set(Some(v));
        }
        if let Some(v) = data.remark {
            active.remark = Set(Some(v));
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
        request: Request<identityv1::DeletePlanRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(identityv1::delete_plan_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        // The cascade the reference's edges draw: quotas and modules go
        // with their plan.
        let quota_ids: Vec<i64> = sys_plan_quotas::Entity::find()
            .filter(sys_plan_quotas::Column::PlanId.eq(id))
            .all(&self.state.db)
            .await
            .map_err(db_status)?
            .into_iter()
            .map(|r| r.id)
            .collect();
        if !quota_ids.is_empty() {
            sys_plan_quotas::Entity::delete_many()
                .filter(sys_plan_quotas::Column::Id.is_in(quota_ids))
                .exec(&self.state.db)
                .await
                .map_err(db_status)?;
        }
        let module_ids: Vec<i64> = sys_plan_modules::Entity::find()
            .filter(sys_plan_modules::Column::PlanId.eq(id))
            .all(&self.state.db)
            .await
            .map_err(db_status)?
            .into_iter()
            .map(|r| r.id)
            .collect();
        if !module_ids.is_empty() {
            sys_plan_modules::Entity::delete_many()
                .filter(sys_plan_modules::Column::Id.is_in(module_ids))
                .exec(&self.state.db)
                .await
                .map_err(db_status)?;
        }
        sys_plans::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

// ── PlanQuota ────────────────────────────────────────────────────────

fn quota_type_name(v: i32) -> Option<&'static str> {
    Some(match identityv1::plan_quota::QuotaType::try_from(v).ok()? {
        identityv1::plan_quota::QuotaType::Storage => "STORAGE",
        identityv1::plan_quota::QuotaType::ApiCall => "API_CALL",
        identityv1::plan_quota::QuotaType::AiTokens => "AI_TOKENS",
        _ => "USER_LIMIT",
    })
}

fn quota_type_proto(name: Option<&str>) -> Option<i32> {
    Some(match name? {
        "STORAGE" => identityv1::plan_quota::QuotaType::Storage as i32,
        "API_CALL" => identityv1::plan_quota::QuotaType::ApiCall as i32,
        "AI_TOKENS" => identityv1::plan_quota::QuotaType::AiTokens as i32,
        _ => identityv1::plan_quota::QuotaType::UserLimit as i32,
    })
}

fn quota_proto(r: sys_plan_quotas::Model) -> identityv1::PlanQuota {
    identityv1::PlanQuota {
        id: Some(r.id),
        plan_id: Some(r.plan_id),
        quota_type: quota_type_proto(r.quota_type.as_deref()),
        quota_value: r.quota_value.map(|v| v as u64),
        created_by: r.created_by,
        updated_by: r.updated_by,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

pub struct PlanQuotaService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl identityv1::plan_quota_service_server::PlanQuotaService for PlanQuotaService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<identityv1::ListPlanQuotaResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sys_plan_quotas::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(identityv1::ListPlanQuotaResponse {
            items: rows.into_iter().map(quota_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<identityv1::CountPlanQuotaResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            sys_plan_quotas::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(identityv1::CountPlanQuotaResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<identityv1::GetPlanQuotaRequest>,
    ) -> Result<Response<identityv1::PlanQuota>, Status> {
        let req = request.into_inner();
        let Some(identityv1::get_plan_quota_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = sys_plan_quotas::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("plan quota not found"))?;
        Ok(Response::new(quota_proto(row)))
    }

    async fn create(
        &self,
        request: Request<identityv1::CreatePlanQuotaRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let plan_id = data.plan_id.ok_or_else(|| bad("data.planId required"))?;
        let active = sys_plan_quotas::ActiveModel {
            plan_id: Set(plan_id),
            quota_type: Set(data.quota_type.and_then(quota_type_name).map(String::from)),
            quota_value: Set(data.quota_value.map(|v| v as i64)),
            created_by: Set(data.created_by),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<identityv1::UpdatePlanQuotaRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let existing = sys_plan_quotas::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("plan quota not found"))?;
        let mut active: sys_plan_quotas::ActiveModel = existing.into();
        if let Some(v) = data.plan_id {
            active.plan_id = Set(v);
        }
        if let Some(v) = data.quota_type {
            active.quota_type = Set(quota_type_name(v).map(String::from));
        }
        if let Some(v) = data.quota_value {
            active.quota_value = Set(Some(v as i64));
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
        request: Request<identityv1::DeletePlanQuotaRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(identityv1::delete_plan_quota_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        sys_plan_quotas::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

// ── PlanModule ───────────────────────────────────────────────────────

fn module_name(v: i32) -> Option<&'static str> {
    Some(match identityv1::Module::try_from(v).ok()? {
        identityv1::Module::Opm => "OPM",
        identityv1::Module::System => "SYSTEM",
        identityv1::Module::Dict => "DICT",
        identityv1::Module::Tenant => "TENANT",
        identityv1::Module::Permission => "PERMISSION",
        identityv1::Module::Log => "LOG",
        identityv1::Module::InternalMessage => "INTERNAL_MESSAGE",
        identityv1::Module::File => "FILE",
        identityv1::Module::Task => "TASK",
        identityv1::Module::Ai => "AI",
        _ => "DASHBOARD",
    })
}

fn module_proto_value(name: Option<&str>) -> Option<i32> {
    Some(match name? {
        "OPM" => identityv1::Module::Opm as i32,
        "SYSTEM" => identityv1::Module::System as i32,
        "DICT" => identityv1::Module::Dict as i32,
        "TENANT" => identityv1::Module::Tenant as i32,
        "PERMISSION" => identityv1::Module::Permission as i32,
        "LOG" => identityv1::Module::Log as i32,
        "INTERNAL_MESSAGE" => identityv1::Module::InternalMessage as i32,
        "FILE" => identityv1::Module::File as i32,
        "TASK" => identityv1::Module::Task as i32,
        "AI" => identityv1::Module::Ai as i32,
        _ => identityv1::Module::Dashboard as i32,
    })
}

fn module_proto(r: sys_plan_modules::Model) -> identityv1::PlanModule {
    identityv1::PlanModule {
        id: Some(r.id),
        plan_id: Some(r.plan_id),
        module: module_proto_value(r.module.as_deref()),
        created_by: r.created_by,
        updated_by: r.updated_by,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

pub struct PlanModuleService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl identityv1::plan_module_service_server::PlanModuleService for PlanModuleService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<identityv1::ListPlanModuleResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sys_plan_modules::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(identityv1::ListPlanModuleResponse {
            items: rows.into_iter().map(module_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<identityv1::CountPlanModuleResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            sys_plan_modules::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(identityv1::CountPlanModuleResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<identityv1::GetPlanModuleRequest>,
    ) -> Result<Response<identityv1::PlanModule>, Status> {
        let req = request.into_inner();
        let Some(identityv1::get_plan_module_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = sys_plan_modules::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("plan module not found"))?;
        Ok(Response::new(module_proto(row)))
    }

    async fn create(
        &self,
        request: Request<identityv1::CreatePlanModuleRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let plan_id = data.plan_id.ok_or_else(|| bad("data.planId required"))?;
        let active = sys_plan_modules::ActiveModel {
            plan_id: Set(plan_id),
            module: Set(data.module.and_then(module_name).map(String::from)),
            created_by: Set(data.created_by),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<identityv1::UpdatePlanModuleRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let existing = sys_plan_modules::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("plan module not found"))?;
        let mut active: sys_plan_modules::ActiveModel = existing.into();
        if let Some(v) = data.plan_id {
            active.plan_id = Set(v);
        }
        if let Some(v) = data.module {
            active.module = Set(module_name(v).map(String::from));
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
        request: Request<identityv1::DeletePlanModuleRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(identityv1::delete_plan_module_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        sys_plan_modules::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}
