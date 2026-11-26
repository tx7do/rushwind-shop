//! The system-config face (the reference's config_service.go) — the
//! key/value parameters the runtime readers resolve. The value_type
//! varchar the golden schema stores (`STRING`/`BOOL`/`INT`) maps onto
//! the proto enum at the face; keys are unique-indexed.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::sys_configs;
use store::paging::fetch_paged;

use shop_proto::proto::config::service::v1 as configv1;
use shop_proto::proto::pagination::PagingRequest;

fn value_type_name(v: i32) -> Option<&'static str> {
    Some(match configv1::config::ConfigValueType::try_from(v).ok()? {
        configv1::config::ConfigValueType::Bool => "BOOL",
        configv1::config::ConfigValueType::Int => "INT",
        _ => "STRING",
    })
}

fn value_type_proto(name: Option<&str>) -> Option<i32> {
    Some(match name? {
        "BOOL" => configv1::config::ConfigValueType::Bool as i32,
        "INT" => configv1::config::ConfigValueType::Int as i32,
        _ => configv1::config::ConfigValueType::String as i32,
    })
}

fn config_proto(r: sys_configs::Model) -> configv1::Config {
    configv1::Config {
        id: Some(r.id),
        name: r.name,
        key: r.key,
        value: r.value,
        value_type: value_type_proto(r.value_type.as_deref()),
        is_built_in: r.is_built_in,
        created_by: r.created_by,
        updated_by: r.updated_by,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

async fn by_id(db: &sea_orm::DatabaseConnection, id: i64) -> Result<sys_configs::Model, Status> {
    sys_configs::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(db_status)?
        .ok_or_else(|| Status::not_found("config not found"))
}

pub struct ConfigService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl configv1::config_service_server::ConfigService for ConfigService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<configv1::ListConfigResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sys_configs::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(configv1::ListConfigResponse {
            items: rows.into_iter().map(config_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<configv1::CountConfigResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            sys_configs::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(configv1::CountConfigResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<configv1::GetConfigRequest>,
    ) -> Result<Response<configv1::Config>, Status> {
        let req = request.into_inner();
        let Some(configv1::get_config_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        Ok(Response::new(config_proto(
            by_id(&self.state.db, id).await?,
        )))
    }

    async fn create(
        &self,
        request: Request<configv1::CreateConfigRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let active = sys_configs::ActiveModel {
            name: Set(data.name),
            key: Set(data.key),
            value: Set(data.value),
            value_type: Set(data.value_type.and_then(value_type_name).map(String::from)),
            is_built_in: Set(data.is_built_in),
            created_by: Set(data.created_by),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<configv1::UpdateConfigRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let existing = by_id(&self.state.db, req.id).await?;
        if existing.is_built_in == Some(true) {
            // The built-in row's key/value ride the release, only the
            // display name stays editable.
            let mut active: sys_configs::ActiveModel = existing.into();
            if let Some(v) = data.name {
                active.name = Set(Some(v));
            }
            active.updated_at = Set(Some(store::now()));
            active.update(&self.state.db).await.map_err(db_status)?;
            return Ok(Response::new(pbjson_types::Empty {}));
        }
        let mut active: sys_configs::ActiveModel = existing.into();
        if let Some(v) = data.name {
            active.name = Set(Some(v));
        }
        if let Some(v) = data.key {
            active.key = Set(Some(v));
        }
        if let Some(v) = data.value {
            active.value = Set(Some(v));
        }
        if let Some(v) = data.value_type {
            active.value_type = Set(value_type_name(v).map(String::from));
        }
        if let Some(v) = data.is_built_in {
            active.is_built_in = Set(Some(v));
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
        request: Request<configv1::DeleteConfigRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(configv1::delete_config_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let existing = by_id(&self.state.db, id).await?;
        if existing.is_built_in == Some(true) {
            return Err(bad("built-in config cannot be deleted"));
        }
        sys_configs::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}
