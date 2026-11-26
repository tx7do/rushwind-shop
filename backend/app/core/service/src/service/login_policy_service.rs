//! The login-policy face — per-tenant authentication policy CRUD (the
//! reference's login_policy_service.go). The varchar enum names the
//! golden schema stores (`BLACKLIST` / `IP` / …) map onto the proto's
//! restriction enums at the face; `allow_missing` on the update turns
//! it into an upsert.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use tonic::{Request, Response, Status};

use crate::data::login_policy_repo as repo;
use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::sys_login_policies;
use store::paging::fetch_paged;

use shop_proto::proto::authentication::service::v1 as authv1;
use shop_proto::proto::pagination::PagingRequest;

/// The proto ordinal → the varchar value name the golden schema
/// stores (short member spellings, the DDL's defaults' shape).
fn type_name(v: i32) -> Option<&'static str> {
    Some(match authv1::login_policy::Type::try_from(v).ok()? {
        authv1::login_policy::Type::Blacklist => "BLACKLIST",
        authv1::login_policy::Type::Whitelist => "WHITELIST",
        _ => return None,
    })
}

fn method_name(v: i32) -> Option<&'static str> {
    Some(match authv1::login_policy::Method::try_from(v).ok()? {
        authv1::login_policy::Method::Ip => "IP",
        authv1::login_policy::Method::Mac => "MAC",
        authv1::login_policy::Method::Region => "REGION",
        authv1::login_policy::Method::Time => "TIME",
        _ => return None,
    })
}

fn login_policy_proto(r: sys_login_policies::Model) -> authv1::LoginPolicy {
    authv1::LoginPolicy {
        id: Some(r.id),
        target_id: r.target_id,
        r#type: r.r#type.as_deref().map(|name| match name {
            "WHITELIST" => authv1::login_policy::Type::Whitelist as i32,
            _ => authv1::login_policy::Type::Blacklist as i32,
        }),
        method: r.method.as_deref().map(|name| match name {
            "MAC" => authv1::login_policy::Method::Mac as i32,
            "REGION" => authv1::login_policy::Method::Region as i32,
            "TIME" => authv1::login_policy::Method::Time as i32,
            _ => authv1::login_policy::Method::Ip as i32,
        }),
        value: r.value,
        reason: r.reason,
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

pub struct LoginPolicyService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl authv1::login_policy_service_server::LoginPolicyService for LoginPolicyService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<authv1::ListLoginPolicyResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sys_login_policies::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(authv1::ListLoginPolicyResponse {
            items: rows.into_iter().map(login_policy_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<authv1::CountLoginPolicyResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            sys_login_policies::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(authv1::CountLoginPolicyResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<authv1::GetLoginPolicyRequest>,
    ) -> Result<Response<authv1::LoginPolicy>, Status> {
        let req = request.into_inner();
        let Some(authv1::get_login_policy_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = repo::login_policies_by_id(&self.state.db, id).await?;
        Ok(Response::new(login_policy_proto(row)))
    }

    async fn create(
        &self,
        request: Request<authv1::CreateLoginPolicyRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let mut a = sys_login_policies::ActiveModel {
            tenant_id: Set(data.tenant_id),
            target_id: Set(data.target_id),
            value: Set(data.value),
            reason: Set(data.reason),
            created_by: Set(data.created_by),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        if let Some(name) = data.r#type.and_then(type_name) {
            a.r#type = Set(Some(name.to_string()));
        }
        if let Some(name) = data.method.and_then(method_name) {
            a.method = Set(Some(name.to_string()));
        }
        repo::insert_login_policies(&self.state.db, a).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<authv1::UpdateLoginPolicyRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        // The upsert face: a missing row materializes through create.
        if req.allow_missing.unwrap_or(false)
            && repo::login_policies_by_id(&self.state.db, req.id)
                .await
                .is_err()
        {
            let mut create_data = data;
            create_data.created_by = create_data.updated_by;
            create_data.updated_by = None;
            self.create(Request::new(authv1::CreateLoginPolicyRequest {
                data: Some(create_data),
            }))
            .await?;
            return Ok(Response::new(pbjson_types::Empty {}));
        }
        let existing = repo::login_policies_by_id(&self.state.db, req.id).await?;
        let mut a: sys_login_policies::ActiveModel = existing.into();
        if let Some(v) = data.target_id {
            a.target_id = Set(Some(v));
        }
        if let Some(name) = data.r#type.and_then(type_name) {
            a.r#type = Set(Some(name.to_string()));
        }
        if let Some(name) = data.method.and_then(method_name) {
            a.method = Set(Some(name.to_string()));
        }
        if let Some(v) = data.value {
            a.value = Set(Some(v));
        }
        if let Some(v) = data.reason {
            a.reason = Set(Some(v));
        }
        if let Some(v) = data.updated_by {
            a.updated_by = Set(Some(v));
        }
        a.updated_at = Set(Some(store::now()));
        a.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<authv1::DeleteLoginPolicyRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(authv1::delete_login_policy_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        repo::delete_login_policies(&self.state.db, id).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}
