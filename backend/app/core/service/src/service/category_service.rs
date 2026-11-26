//! The category service — CRUD with the translation sub-tables
//! (the reference's category_service.go). The tree composition
//! (children) stays to the client: List/Get return flat rows with
//! parent_id/depth/path carried (the reference returns the same
//! flat rows over paged listings).

use std::sync::Arc;

use sea_orm::{EntityTrait, Set};
use tonic::{Request, Response, Status};

use crate::data::category_repo as repo;
use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{categories, category_translations};
use store::paging::fetch_paged;

use shop_proto::proto::catalog::service::v1 as catalogv1;
use shop_proto::proto::pagination::PagingRequest;

pub(crate) fn category_translation_proto(
    r: category_translations::Model,
) -> catalogv1::CategoryTranslation {
    catalogv1::CategoryTranslation {
        id: Some(r.id),
        category_id: Some(r.category_id),
        language_code: Some(r.language_code),
        name: Some(r.name),
        slug: r.slug,
        description: r.description,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

pub(crate) fn category_proto(
    r: categories::Model,
    translations: Vec<category_translations::Model>,
) -> catalogv1::Category {
    let available_languages: Vec<String> = translations
        .iter()
        .map(|t| t.language_code.clone())
        .collect();
    catalogv1::Category {
        id: Some(r.id),
        sort_order: r.sort_order.map(|v| v as u32),
        image_url: r.image_url,
        translations: translations
            .into_iter()
            .map(category_translation_proto)
            .collect(),
        available_languages,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

async fn category_translations_of(
    db: &sea_orm::DatabaseConnection,
    id: i64,
) -> Result<Vec<category_translations::Model>, Status> {
    repo::translations_of(db, id).await
}

async fn category_of(
    db: &sea_orm::DatabaseConnection,
    id: i64,
) -> Result<catalogv1::Category, Status> {
    let row = repo::category_by_id(db, id).await?;
    let translations = category_translations_of(db, row.id).await?;
    Ok(category_proto(row, translations))
}

/// The main-table write out of the proto `Brand` data payload (the
/// translation rows ride their own methods — the reference's service
/// keeps the same split).
fn category_active(
    data: catalogv1::Category,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> categories::ActiveModel {
    categories::ActiveModel {
        sort_order: data
            .sort_order
            .map(|v| Set(Some(v as i32)))
            .unwrap_or(Set(None::<i32>)),
        path: Set(data.path),
        depth: Set(data.depth),
        parent_id: Set(data.parent_id),
        image_url: Set(data.image_url),
        created_at: Set(Some(now)),
        ..Default::default()
    }
}

pub struct CategoryService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl catalogv1::category_service_server::CategoryService for CategoryService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::ListCategoryResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            categories::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        let mut items = Vec::with_capacity(rows.len());
        for r in rows {
            let translations = category_translations_of(&self.state.db, r.id).await?;
            items.push(category_proto(r, translations));
        }
        Ok(Response::new(catalogv1::ListCategoryResponse {
            items,
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::CountCategoryResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            categories::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::CountCategoryResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<catalogv1::GetCategoryRequest>,
    ) -> Result<Response<catalogv1::Category>, Status> {
        let req = request.into_inner();
        let row = match req.query_by {
            Some(catalogv1::get_category_request::QueryBy::Id(id)) => {
                category_of(&self.state.db, id).await?
            }
            Some(catalogv1::get_category_request::QueryBy::Code(code)) => {
                let row = repo::category_by_code(&self.state.db, &code).await?;
                let translations = category_translations_of(&self.state.db, row.id).await?;
                category_proto(row, translations)
            }
            None => return Err(bad("query_by required")),
        };
        Ok(Response::new(row))
    }

    async fn create(
        &self,
        request: Request<catalogv1::CreateCategoryRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let mut data = req.data.ok_or_else(|| bad("data required"))?;
        let now = store::now();
        let translations = std::mem::take(&mut data.translations);
        let active = category_active(data, now);
        let row = repo::insert_category(&self.state.db, active).await?;
        // The payload's translation rows land with the brand (the
        // reference's create carries the same dual write).
        for t in translations {
            let ta = category_translation_active(t, row.id, now)?;
            repo::insert_translation(&self.state.db, ta).await?;
        }
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn batch_create(
        &self,
        request: Request<catalogv1::BatchCreateCategoriesRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let now = store::now();
        let mut created = Vec::with_capacity(req.items.len());
        for data in req.items {
            let active = category_active(data, now);
            let row = repo::insert_category(&self.state.db, active).await?;
            created.push(row.id);
        }
        let _ = created;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<catalogv1::UpdateCategoryRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = repo::category_by_id(&self.state.db, req.id).await?;
        let mut active: categories::ActiveModel = existing.into();
        if let Some(v) = data.sort_order {
            active.sort_order = Set(Some(v as i32));
        }
        active.path = Set(data.path);
        active.depth = Set(data.depth);
        active.parent_id = Set(data.parent_id);
        active.image_url = Set(data.image_url);
        active.updated_at = Set(Some(store::now()));
        repo::update_category(&self.state.db, active).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<catalogv1::DeleteCategoryRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::delete_category_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        repo::delete_category(&self.state.db, id).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn translation_exists(
        &self,
        request: Request<catalogv1::CategoryTranslationExistsRequest>,
    ) -> Result<Response<catalogv1::CategoryTranslationExistsResponse>, Status> {
        let req = request.into_inner();
        let row = repo::translation_of(&self.state.db, req.category_id, &req.language_code).await?;
        Ok(Response::new(
            catalogv1::CategoryTranslationExistsResponse {
                exists: row.is_some(),
            },
        ))
    }

    async fn get_translation(
        &self,
        request: Request<catalogv1::GetCategoryRequest>,
    ) -> Result<Response<catalogv1::CategoryTranslation>, Status> {
        let req = request.into_inner();
        let id = match req.query_by {
            Some(catalogv1::get_category_request::QueryBy::Id(id)) => id,
            _ => return Err(bad("query_by.id required")),
        };
        let locale = req
            .locale
            .filter(|l| !l.is_empty())
            .ok_or_else(|| bad("locale required"))?;
        let row = repo::translation_of(&self.state.db, id, &locale)
            .await?
            .ok_or_else(|| tonic::Status::not_found("brand translation not found"))?;
        Ok(Response::new(category_translation_proto(row)))
    }

    async fn create_translation(
        &self,
        request: Request<catalogv1::CreateCategoryTranslationRequest>,
    ) -> Result<Response<catalogv1::CategoryTranslation>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let brand_id = data
            .category_id
            .ok_or_else(|| bad("data.categoryId required"))?;
        let active = category_translation_active(data, brand_id, store::now())?;
        let row = repo::insert_translation(&self.state.db, active).await?;
        Ok(Response::new(category_translation_proto(row)))
    }

    async fn update_translation(
        &self,
        request: Request<catalogv1::UpdateCategoryTranslationRequest>,
    ) -> Result<Response<catalogv1::CategoryTranslation>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = repo::translation_by_id(&self.state.db, req.id).await?;
        let mut active: category_translations::ActiveModel = existing.into();
        if let Some(v) = data.name {
            active.name = Set(v);
        }
        active.slug = Set(data.slug);
        active.description = Set(data.description);
        active.full_path = Set(data.full_path);
        active.updated_at = Set(Some(store::now()));
        let row = repo::update_translation(&self.state.db, active).await?;
        Ok(Response::new(category_translation_proto(row)))
    }

    async fn delete_translation(
        &self,
        request: Request<catalogv1::DeleteCategoryTranslationRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let id = match req.query_by {
            Some(catalogv1::delete_category_translation_request::QueryBy::Id(id)) => id,
            Some(catalogv1::delete_category_translation_request::QueryBy::Identifier(
                identifier,
            )) => {
                let row = repo::translation_of(
                    &self.state.db,
                    identifier.category_id,
                    &identifier.language_code,
                )
                .await?
                .ok_or_else(|| tonic::Status::not_found("brand translation not found"))?;
                row.id
            }
            None => return Err(bad("query_by required")),
        };
        repo::delete_translation(&self.state.db, id).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

/// The required triple (categoryId, languageCode, name) resolved by the
/// caller — the entity's NOT NULL columns.
fn category_translation_active(
    data: catalogv1::CategoryTranslation,
    category_id: i64,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Result<category_translations::ActiveModel, Status> {
    let language_code = data
        .language_code
        .filter(|l| !l.is_empty())
        .ok_or_else(|| bad("data.languageCode required"))?;
    let name = data
        .name
        .filter(|n| !n.is_empty())
        .ok_or_else(|| bad("data.name required"))?;
    Ok(category_translations::ActiveModel {
        category_id: Set(category_id),
        language_code: Set(language_code),
        name: Set(name),
        slug: Set(data.slug),
        description: Set(data.description),
        full_path: Set(data.full_path),
        created_at: Set(Some(now)),
        ..Default::default()
    })
}

// db_status stays referenced for the repo layer's error mapping.
#[allow(dead_code)]
fn _keep(e: sea_orm::DbErr) -> Status {
    db_status(e)
}
