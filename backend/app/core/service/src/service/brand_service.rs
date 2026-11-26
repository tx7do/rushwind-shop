//! The brand service — CRUD with the translation sub-tables
//! (the reference's brand_service.go).

use std::sync::Arc;

use sea_orm::{EntityTrait, Set};
use tonic::{Request, Response, Status};

use crate::data::brand_repo as repo;
use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{brand_translations, brands};
use store::paging::fetch_paged;

use shop_proto::proto::catalog::service::v1 as catalogv1;
use shop_proto::proto::pagination::PagingRequest;

pub(crate) fn brand_translation_proto(r: brand_translations::Model) -> catalogv1::BrandTranslation {
    catalogv1::BrandTranslation {
        id: Some(r.id),
        brand_id: Some(r.brand_id),
        language_code: Some(r.language_code),
        name: Some(r.name),
        slug: r.slug,
        description: r.description,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

pub(crate) fn brand_proto(
    r: brands::Model,
    translations: Vec<brand_translations::Model>,
) -> catalogv1::Brand {
    let available_languages: Vec<String> = translations
        .iter()
        .map(|t| t.language_code.clone())
        .collect();
    catalogv1::Brand {
        id: Some(r.id),
        sort_order: r.sort_order.map(|v| v as u32),
        logo_url: r.logo_url,
        translations: translations
            .into_iter()
            .map(brand_translation_proto)
            .collect(),
        available_languages,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

async fn brand_translations_of(
    db: &sea_orm::DatabaseConnection,
    id: i64,
) -> Result<Vec<brand_translations::Model>, Status> {
    repo::translations_of(db, id).await
}

async fn brand_of(db: &sea_orm::DatabaseConnection, id: i64) -> Result<catalogv1::Brand, Status> {
    let row = repo::brand_by_id(db, id).await?;
    let translations = brand_translations_of(db, row.id).await?;
    Ok(brand_proto(row, translations))
}

/// The main-table write out of the proto `Brand` data payload (the
/// translation rows ride their own methods — the reference's service
/// keeps the same split).
fn brand_active(
    data: catalogv1::Brand,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> brands::ActiveModel {
    brands::ActiveModel {
        sort_order: data
            .sort_order
            .map(|v| Set(Some(v as i32)))
            .unwrap_or(Set(None::<i32>)),
        logo_url: Set(data.logo_url),
        created_at: Set(Some(now)),
        ..Default::default()
    }
}

pub struct BrandService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl catalogv1::brand_service_server::BrandService for BrandService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::ListBrandResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            brands::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        let mut items = Vec::with_capacity(rows.len());
        for r in rows {
            let translations = brand_translations_of(&self.state.db, r.id).await?;
            items.push(brand_proto(r, translations));
        }
        Ok(Response::new(catalogv1::ListBrandResponse {
            items,
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::CountBrandResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            brands::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::CountBrandResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<catalogv1::GetBrandRequest>,
    ) -> Result<Response<catalogv1::Brand>, Status> {
        let req = request.into_inner();
        let row = match req.query_by {
            Some(catalogv1::get_brand_request::QueryBy::Id(id)) => {
                brand_of(&self.state.db, id).await?
            }
            Some(catalogv1::get_brand_request::QueryBy::Code(code)) => {
                let row = repo::brand_by_code(&self.state.db, &code).await?;
                let translations = brand_translations_of(&self.state.db, row.id).await?;
                brand_proto(row, translations)
            }
            None => return Err(bad("query_by required")),
        };
        Ok(Response::new(row))
    }

    async fn create(
        &self,
        request: Request<catalogv1::CreateBrandRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let mut data = req.data.ok_or_else(|| bad("data required"))?;
        let now = store::now();
        let translations = std::mem::take(&mut data.translations);
        let active = brand_active(data, now);
        let row = repo::insert_brand(&self.state.db, active).await?;
        // The payload's translation rows land with the brand (the
        // reference's create carries the same dual write).
        for t in translations {
            let ta = brand_translation_active(t, row.id, now)?;
            repo::insert_translation(&self.state.db, ta).await?;
        }
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn batch_create(
        &self,
        request: Request<catalogv1::BatchCreateBrandsRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let now = store::now();
        let mut created = Vec::with_capacity(req.items.len());
        for data in req.items {
            let active = brand_active(data, now);
            let row = repo::insert_brand(&self.state.db, active).await?;
            created.push(row.id);
        }
        let _ = created;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<catalogv1::UpdateBrandRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = repo::brand_by_id(&self.state.db, req.id).await?;
        let mut active: brands::ActiveModel = existing.into();
        if let Some(v) = data.sort_order {
            active.sort_order = Set(Some(v as i32));
        }
        active.logo_url = Set(data.logo_url);
        active.updated_at = Set(Some(store::now()));
        repo::update_brand(&self.state.db, active).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<catalogv1::DeleteBrandRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::delete_brand_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        repo::delete_brand(&self.state.db, id).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn translation_exists(
        &self,
        request: Request<catalogv1::BrandTranslationExistsRequest>,
    ) -> Result<Response<catalogv1::BrandTranslationExistsResponse>, Status> {
        let req = request.into_inner();
        let row = repo::translation_of(&self.state.db, req.brand_id, &req.language_code).await?;
        Ok(Response::new(catalogv1::BrandTranslationExistsResponse {
            exists: row.is_some(),
        }))
    }

    async fn get_translation(
        &self,
        request: Request<catalogv1::GetBrandRequest>,
    ) -> Result<Response<catalogv1::BrandTranslation>, Status> {
        let req = request.into_inner();
        let id = match req.query_by {
            Some(catalogv1::get_brand_request::QueryBy::Id(id)) => id,
            _ => return Err(bad("query_by.id required")),
        };
        let locale = req
            .locale
            .filter(|l| !l.is_empty())
            .ok_or_else(|| bad("locale required"))?;
        let row = repo::translation_of(&self.state.db, id, &locale)
            .await?
            .ok_or_else(|| tonic::Status::not_found("brand translation not found"))?;
        Ok(Response::new(brand_translation_proto(row)))
    }

    async fn create_translation(
        &self,
        request: Request<catalogv1::CreateBrandTranslationRequest>,
    ) -> Result<Response<catalogv1::BrandTranslation>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let brand_id = data.brand_id.ok_or_else(|| bad("data.brandId required"))?;
        let active = brand_translation_active(data, brand_id, store::now())?;
        let row = repo::insert_translation(&self.state.db, active).await?;
        Ok(Response::new(brand_translation_proto(row)))
    }

    async fn update_translation(
        &self,
        request: Request<catalogv1::UpdateBrandTranslationRequest>,
    ) -> Result<Response<catalogv1::BrandTranslation>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = repo::translation_by_id(&self.state.db, req.id).await?;
        let mut active: brand_translations::ActiveModel = existing.into();
        if let Some(v) = data.name {
            active.name = Set(v);
        }
        active.slug = Set(data.slug);
        active.description = Set(data.description);
        active.updated_at = Set(Some(store::now()));
        let row = repo::update_translation(&self.state.db, active).await?;
        Ok(Response::new(brand_translation_proto(row)))
    }

    async fn delete_translation(
        &self,
        request: Request<catalogv1::DeleteBrandTranslationRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let id = match req.query_by {
            Some(catalogv1::delete_brand_translation_request::QueryBy::Id(id)) => id,
            Some(catalogv1::delete_brand_translation_request::QueryBy::Identifier(identifier)) => {
                let row = repo::translation_of(
                    &self.state.db,
                    identifier.brand_id,
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

/// The required triple (brandId, languageCode, name) resolved by the
/// caller — the entity's NOT NULL columns.
fn brand_translation_active(
    data: catalogv1::BrandTranslation,
    brand_id: i64,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Result<brand_translations::ActiveModel, Status> {
    let language_code = data
        .language_code
        .filter(|l| !l.is_empty())
        .ok_or_else(|| bad("data.languageCode required"))?;
    let name = data
        .name
        .filter(|n| !n.is_empty())
        .ok_or_else(|| bad("data.name required"))?;
    Ok(brand_translations::ActiveModel {
        brand_id: Set(brand_id),
        language_code: Set(language_code),
        name: Set(name),
        slug: Set(data.slug),
        description: Set(data.description),
        created_at: Set(Some(now)),
        ..Default::default()
    })
}

// db_status stays referenced for the repo layer's error mapping.
#[allow(dead_code)]
fn _keep(e: sea_orm::DbErr) -> Status {
    db_status(e)
}
