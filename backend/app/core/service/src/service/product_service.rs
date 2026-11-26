//! The product service — CRUD with the translation sub-tables
//! (the reference's product_service.go) plus the two search RPCs.

use std::sync::Arc;

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::data::product_repo as repo;
use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{product_translations, products};
use store::paging::fetch_paged;

use shop_proto::proto::catalog::service::v1 as catalogv1;
use shop_proto::proto::pagination::PagingRequest;

pub(crate) fn product_translation_proto(
    r: product_translations::Model,
) -> catalogv1::ProductTranslation {
    catalogv1::ProductTranslation {
        id: Some(r.id),
        product_id: Some(r.product_id),
        language_code: Some(r.language_code),
        name: Some(r.name),
        slug: r.slug,
        short_description: r.short_description,
        long_description: r.long_description,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

pub(crate) fn product_proto(
    r: products::Model,
    translations: Vec<product_translations::Model>,
) -> catalogv1::Product {
    let available_languages: Vec<String> = translations
        .iter()
        .map(|t| t.language_code.clone())
        .collect();
    catalogv1::Product {
        id: Some(r.id),
        sort_order: r.sort_order.map(|v| v as u32),
        image_url: r.image_url,
        status: product_status_to_i32(r.status),
        category_id: r.category_id,
        brand_id: r.brand_id,
        translations: translations
            .into_iter()
            .map(product_translation_proto)
            .collect(),
        available_languages,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

async fn product_translations_of(
    db: &sea_orm::DatabaseConnection,
    id: i64,
) -> Result<Vec<product_translations::Model>, Status> {
    repo::translations_of(db, id).await
}

async fn product_of(
    db: &sea_orm::DatabaseConnection,
    id: i64,
) -> Result<catalogv1::Product, Status> {
    let row = repo::product_by_id(db, id).await?;
    let translations = product_translations_of(db, row.id).await?;
    Ok(product_proto(row, translations))
}

/// The main-table write out of the proto `Product` data payload (the
/// translation rows ride their own methods — the reference's service
/// keeps the same split).
fn product_active(
    data: catalogv1::Product,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> products::ActiveModel {
    products::ActiveModel {
        sort_order: data
            .sort_order
            .map(|v| Set(Some(v as i32)))
            .unwrap_or(Set(None::<i32>)),
        image_url: Set(data.image_url),
        status: Set(data.status.map(product_status_to_str)),
        category_id: Set(data.category_id),
        brand_id: Set(data.brand_id),
        created_at: Set(Some(now.into())),
        ..Default::default()
    }
}

pub struct ProductService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl catalogv1::product_service_server::ProductService for ProductService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::ListProductResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            products::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        let mut items = Vec::with_capacity(rows.len());
        for r in rows {
            let translations = product_translations_of(&self.state.db, r.id).await?;
            items.push(product_proto(r, translations));
        }
        Ok(Response::new(catalogv1::ListProductResponse {
            items,
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::CountProductResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            products::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::CountProductResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<catalogv1::GetProductRequest>,
    ) -> Result<Response<catalogv1::Product>, Status> {
        let req = request.into_inner();
        let row = match req.query_by {
            Some(catalogv1::get_product_request::QueryBy::Id(id)) => {
                product_of(&self.state.db, id).await?
            }
            Some(catalogv1::get_product_request::QueryBy::Code(code)) => {
                let row = repo::product_by_code(&self.state.db, &code).await?;
                let translations = product_translations_of(&self.state.db, row.id).await?;
                product_proto(row, translations)
            }
            None => return Err(bad("query_by required")),
        };
        Ok(Response::new(row))
    }

    async fn create(
        &self,
        request: Request<catalogv1::CreateProductRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let mut data = req.data.ok_or_else(|| bad("data required"))?;
        let now = store::now();
        let translations = std::mem::take(&mut data.translations);
        let active = product_active(data, now);
        let row = repo::insert_product(&self.state.db, active).await?;
        // The payload's translation rows land with the brand (the
        // reference's create carries the same dual write).
        for t in translations {
            let ta = product_translation_active(t, row.id, now)?;
            repo::insert_translation(&self.state.db, ta).await?;
        }
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn batch_create(
        &self,
        request: Request<catalogv1::BatchCreateProductsRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let now = store::now();
        let mut created = Vec::with_capacity(req.items.len());
        for data in req.items {
            let active = product_active(data, now);
            let row = repo::insert_product(&self.state.db, active).await?;
            created.push(row.id);
        }
        let _ = created;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<catalogv1::UpdateProductRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = repo::product_by_id(&self.state.db, req.id).await?;
        let mut active: products::ActiveModel = existing.into();
        if let Some(v) = data.sort_order {
            active.sort_order = Set(Some(v as i32));
        }
        active.image_url = Set(data.image_url);
        active.status = Set(data.status.map(product_status_to_str));
        active.category_id = Set(data.category_id);
        active.brand_id = Set(data.brand_id);
        active.updated_at = Set(Some(store::now().into()));
        repo::update_product(&self.state.db, active).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<catalogv1::DeleteProductRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::delete_product_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        repo::delete_product(&self.state.db, id).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn translation_exists(
        &self,
        request: Request<catalogv1::ProductTranslationExistsRequest>,
    ) -> Result<Response<catalogv1::ProductTranslationExistsResponse>, Status> {
        let req = request.into_inner();
        let row = repo::translation_of(&self.state.db, req.product_id, &req.language_code).await?;
        Ok(Response::new(catalogv1::ProductTranslationExistsResponse {
            exists: row.is_some(),
        }))
    }

    async fn get_translation(
        &self,
        request: Request<catalogv1::GetProductRequest>,
    ) -> Result<Response<catalogv1::ProductTranslation>, Status> {
        let req = request.into_inner();
        let id = match req.query_by {
            Some(catalogv1::get_product_request::QueryBy::Id(id)) => id,
            _ => return Err(bad("query_by.id required")),
        };
        let locale = req
            .locale
            .filter(|l| !l.is_empty())
            .ok_or_else(|| bad("locale required"))?;
        let row = repo::translation_of(&self.state.db, id, &locale)
            .await?
            .ok_or_else(|| tonic::Status::not_found("brand translation not found"))?;
        Ok(Response::new(product_translation_proto(row)))
    }

    async fn create_translation(
        &self,
        request: Request<catalogv1::CreateProductTranslationRequest>,
    ) -> Result<Response<catalogv1::ProductTranslation>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let brand_id = data
            .product_id
            .ok_or_else(|| bad("data.productId required"))?;
        let active = product_translation_active(data, brand_id, store::now())?;
        let row = repo::insert_translation(&self.state.db, active).await?;
        Ok(Response::new(product_translation_proto(row)))
    }

    async fn update_translation(
        &self,
        request: Request<catalogv1::UpdateProductTranslationRequest>,
    ) -> Result<Response<catalogv1::ProductTranslation>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = repo::translation_by_id(&self.state.db, req.id).await?;
        let mut active: product_translations::ActiveModel = existing.into();
        if let Some(v) = data.name {
            active.name = Set(v);
        }
        active.slug = Set(data.slug);
        active.short_description = Set(data.short_description);
        active.long_description = Set(data.long_description);
        active.updated_at = Set(Some(store::now().into()));
        let row = repo::update_translation(&self.state.db, active).await?;
        Ok(Response::new(product_translation_proto(row)))
    }

    async fn delete_translation(
        &self,
        request: Request<catalogv1::DeleteProductTranslationRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let id = match req.query_by {
            Some(catalogv1::delete_product_translation_request::QueryBy::Id(id)) => id,
            Some(catalogv1::delete_product_translation_request::QueryBy::Identifier(
                identifier,
            )) => {
                let row = repo::translation_of(
                    &self.state.db,
                    identifier.product_id,
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
    /// The storefront search: an ILIKE over the requested language's
    /// translation names (the products' status must be ACTIVE), joined
    /// to the main row for the image. Paged off the request's own
    /// page/page_size (server cap 50).
    async fn search_products(
        &self,
        request: Request<catalogv1::SearchProductsRequest>,
    ) -> Result<Response<catalogv1::SearchProductsResponse>, Status> {
        let req = request.into_inner();
        if req.query.trim().is_empty() {
            return Err(bad("query required"));
        }
        let language = if req.language.is_empty() {
            "zh-CN".to_string()
        } else {
            req.language
        };
        let page = req.page.max(0) as u64;
        let page_size = (req.page_size.max(1) as u64).min(50);
        let like = format!("%{}%", req.query.trim());
        let translations = product_translations::Entity::find()
            .filter(product_translations::Column::LanguageCode.eq(language.clone()))
            .filter(product_translations::Column::Name.ilike(&like))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let ids: Vec<i64> = translations.iter().map(|t| t.product_id).collect();
        let products = products::Entity::find()
            .filter(products::Column::Id.is_in(ids))
            .filter(products::Column::Status.eq("PRODUCT_STATUS_ACTIVE"))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let by_id: std::collections::HashMap<i64, products::Model> =
            products.into_iter().map(|p| (p.id, p)).collect();
        let hits: Vec<(product_translations::Model, products::Model)> = translations
            .into_iter()
            .filter_map(|t| by_id.get(&t.product_id).cloned().map(|p| (t, p)))
            .collect();
        let total = hits.len() as i32;
        let items: Vec<catalogv1::SearchProductHit> = hits
            .into_iter()
            .skip(page as usize * page_size as usize)
            .take(page_size as usize)
            .map(|(t, p)| catalogv1::SearchProductHit {
                product_id: p.id,
                language: language.clone(),
                name: t.name,
                image_url: p.image_url.unwrap_or_default(),
            })
            .collect();
        Ok(Response::new(catalogv1::SearchProductsResponse {
            items,
            total,
        }))
    }

    /// The suggestions: the same face, capped at five, carrying the
    /// translation name and the image only.
    async fn search_product_suggestions(
        &self,
        request: Request<catalogv1::SearchProductSuggestionsRequest>,
    ) -> Result<Response<catalogv1::SearchProductSuggestionsResponse>, Status> {
        let req = request.into_inner();
        if req.query.trim().is_empty() {
            return Err(bad("query required"));
        }
        let like = format!("%{}%", req.query.trim());
        let translations = product_translations::Entity::find()
            .filter(product_translations::Column::Name.ilike(&like))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let ids: Vec<i64> = translations.iter().map(|t| t.product_id).collect();
        let products = products::Entity::find()
            .filter(products::Column::Id.is_in(ids))
            .filter(products::Column::Status.eq("PRODUCT_STATUS_ACTIVE"))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let by_id: std::collections::HashMap<i64, products::Model> =
            products.into_iter().map(|p| (p.id, p)).collect();
        let items: Vec<catalogv1::search_product_suggestions_response::Suggestion> = translations
            .into_iter()
            .filter_map(|t| {
                by_id.get(&t.product_id).map(|p| {
                    catalogv1::search_product_suggestions_response::Suggestion {
                        product_id: p.id,
                        name: t.name.clone(),
                        image_url: p.image_url.clone().unwrap_or_default(),
                    }
                })
            })
            .take(5)
            .collect();
        Ok(Response::new(catalogv1::SearchProductSuggestionsResponse {
            items,
        }))
    }
}

/// The required triple (productId, languageCode, name) resolved by the
/// caller — the entity's NOT NULL columns.
fn product_translation_active(
    data: catalogv1::ProductTranslation,
    product_id: i64,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Result<product_translations::ActiveModel, Status> {
    let language_code = data
        .language_code
        .filter(|l| !l.is_empty())
        .ok_or_else(|| bad("data.languageCode required"))?;
    let name = data
        .name
        .filter(|n| !n.is_empty())
        .ok_or_else(|| bad("data.name required"))?;
    Ok(product_translations::ActiveModel {
        product_id: Set(product_id),
        language_code: Set(language_code),
        name: Set(name),
        slug: Set(data.slug),
        short_description: Set(data.short_description),
        long_description: Set(data.long_description),
        created_at: Set(Some(now.into())),
        ..Default::default()
    })
}

/// The proto ProductStatus enum ↔ the stored spelling (the demo-data
/// INSERT pins `PRODUCT_STATUS_ACTIVE` as the column's form).
fn product_status_to_i32(stored: Option<String>) -> Option<i32> {
    use catalogv1::product::ProductStatus;
    Some(match stored.as_deref() {
        Some("PRODUCT_STATUS_DRAFT") => ProductStatus::Draft,
        Some("PRODUCT_STATUS_INACTIVE") => ProductStatus::Inactive,
        _ => ProductStatus::Active,
    } as i32)
}

fn product_status_to_str(v: i32) -> String {
    use catalogv1::product::ProductStatus;
    match ProductStatus::try_from(v) {
        Ok(ProductStatus::Draft) => "PRODUCT_STATUS_DRAFT".to_string(),
        Ok(ProductStatus::Inactive) => "PRODUCT_STATUS_INACTIVE".to_string(),
        _ => "PRODUCT_STATUS_ACTIVE".to_string(),
    }
}
