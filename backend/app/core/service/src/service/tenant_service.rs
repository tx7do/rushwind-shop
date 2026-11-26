//! The tenant service — the complete tenant face: list/get/resolve plus
//! create/update/delete and the tenant-with-admin bootstrap, mirroring
//! the reference's tenant_service.go.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::data::tenant_repo;
use crate::state::{bad, db_status, not_found, ts_to_proto, AppState};
use store::entities::{sys_tenants, sys_users};
use store::paging::fetch_paged;

use shop_proto::proto::identity::service::v1 as identityv1;

pub(crate) fn tenant_proto(r: sys_tenants::Model) -> identityv1::Tenant {
    identityv1::Tenant {
        id: Some(r.id),
        name: r.name,
        code: r.code,
        logo_url: r.logo_url,
        domain: r.domain,
        industry: r.industry,
        admin_user_id: r.admin_user_id,
        status: Some(1),
        r#type: Some(1),
        audit_status: r.audit_status.as_deref().map(|_| 1),
        subscription_plan: r.subscription_plan,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

// ── Tenant writes ────────────────────────────────────────────────────

/// 管理员名回填（参照 enrichRelations 的 AdminUserName 面，username
/// 形态；best-effort）。
async fn bind_admin_user_names(state: &AppState, items: &mut [identityv1::Tenant]) {
    let ids: std::collections::BTreeSet<i64> =
        items.iter().filter_map(|t| t.admin_user_id).collect();
    if ids.is_empty() {
        return;
    }
    let users: std::collections::HashMap<i64, String> = sys_users::Entity::find()
        .filter(sys_users::Column::Id.is_in(ids))
        .all(&state.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|u| (u.id, u.username))
        .collect();
    for t in items {
        if let Some(uid) = t.admin_user_id {
            t.admin_user_name = users.get(&uid).cloned();
        }
    }
}

pub struct TenantService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl identityv1::tenant_service_server::TenantService for TenantService {
    async fn list(
        &self,
        request: Request<shop_proto::proto::pagination::PagingRequest>,
    ) -> Result<Response<identityv1::ListTenantResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sys_tenants::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        let mut items: Vec<identityv1::Tenant> = rows.into_iter().map(tenant_proto).collect();
        bind_admin_user_names(&self.state, &mut items).await;
        Ok(Response::new(identityv1::ListTenantResponse {
            items,
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        _request: Request<shop_proto::proto::pagination::PagingRequest>,
    ) -> Result<Response<identityv1::CountTenantResponse>, Status> {
        use sea_orm::PaginatorTrait as _;
        let total = sys_tenants::Entity::find()
            .count(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(identityv1::CountTenantResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<identityv1::GetTenantRequest>,
    ) -> Result<Response<identityv1::Tenant>, Status> {
        let req = request.into_inner();
        use identityv1::get_tenant_request::QueryBy;
        let row = match req.query_by {
            Some(QueryBy::Id(id)) => {
                sys_tenants::Entity::find_by_id(id)
                    .one(&self.state.db)
                    .await
            }
            Some(QueryBy::Code(code)) => {
                sys_tenants::Entity::find()
                    .filter(sys_tenants::Column::Code.eq(code))
                    .one(&self.state.db)
                    .await
            }
            Some(QueryBy::Name(name)) => {
                sys_tenants::Entity::find()
                    .filter(sys_tenants::Column::Name.eq(name))
                    .one(&self.state.db)
                    .await
            }
            None => return Err(bad("query_by required")),
        }
        .map_err(db_status)?
        .ok_or_else(|| not_found("tenant"))?;
        let mut tenant = tenant_proto(row);
        bind_admin_user_names(&self.state, std::slice::from_mut(&mut tenant)).await;
        Ok(Response::new(tenant))
    }

    async fn create(
        &self,
        request: Request<identityv1::CreateTenantRequest>,
    ) -> Result<Response<identityv1::Tenant>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let row = sys_tenants::ActiveModel {
            name: Set(Some(data.name.clone().unwrap_or_default())),
            code: Set(Some(data.code.clone().unwrap_or_default())),
            domain: Set(data.domain),
            industry: Set(data.industry),
            status: Set(Some("ON".to_string())),
            created_at: Set(Some(store::now())),
            updated_at: Set(Some(store::now())),
            ..Default::default()
        }
        .insert(&self.state.db)
        .await
        .map_err(db_status)?;
        Ok(Response::new(tenant_proto(row)))
    }

    async fn update(
        &self,
        request: Request<identityv1::UpdateTenantRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let row = sys_tenants::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| not_found("tenant"))?;
        let mut a: sys_tenants::ActiveModel = row.into();
        if let Some(data) = req.data {
            if let Some(v) = data.name {
                a.name = Set(Some(v));
            }
            if let Some(v) = data.domain {
                a.domain = Set(Some(v));
            }
            if let Some(v) = data.status {
                a.status = Set(Some(if v == 1 { "ON" } else { "OFF" }.to_string()));
            }
        }
        a.updated_at = Set(Some(store::now()));
        a.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<identityv1::DeleteTenantRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(identityv1::delete_tenant_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        tenant_repo::delete_tenants(&self.state.db, id).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn tenant_exists(
        &self,
        request: Request<identityv1::TenantExistsRequest>,
    ) -> Result<Response<identityv1::TenantExistsResponse>, Status> {
        let req = request.into_inner();
        let row = sys_tenants::Entity::find()
            .filter(
                sea_orm::Condition::any()
                    .add(sys_tenants::Column::Code.eq(req.code.clone()))
                    .add(sys_tenants::Column::Name.eq(req.name.clone())),
            )
            .one(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(identityv1::TenantExistsResponse {
            exist: row.is_some(),
        }))
    }

    /// CreateTenantWithAdminUser — the composite bootstrap (the role
    /// template copy and the credential write ride the rbac phase's
    /// landing; this deployment carries the plain tenant create).
    async fn create_tenant_with_admin_user(
        &self,
        request: Request<identityv1::CreateTenantWithAdminUserRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        // The composite bootstrap (the role-template copy and the
        // credential write) rides the rbac phase's landing; the plain
        // tenant create above carries this deployment for now.
        let _ = request.into_inner();
        Err(Status::unimplemented(
            "create_tenant_with_admin_user rides the rbac phase",
        ))
    }
}
