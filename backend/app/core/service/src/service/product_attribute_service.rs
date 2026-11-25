//! The product-attribute services — the attribute definitions and
//! their values, each with the translation sub-tables (the reference's
//! product_attribute_service.go / product_attribute_value_service.go).
//! The values nest under the attribute (the sku's option axis); the
//! translation rows carry the per-language names.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::data::attr_repo as repo;
use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{
    product_attribute_translations, product_attribute_values, product_attributes,
};
use store::paging::fetch_paged;

use shop_proto::proto::catalog::service::v1 as catalogv1;

use shop_proto::proto::pagination::PagingRequest;

pub(crate) fn attr_translation_proto(
    r: product_attribute_translations::Model,
) -> catalogv1::ProductAttributeTranslation {
    catalogv1::ProductAttributeTranslation {
        id: Some(r.id),
        attribute_id: None,
        language_code: Some(r.language_code),
        name: Some(r.name),
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

pub(crate) fn attribute_proto(
    r: product_attributes::Model,
    translations: Vec<product_attribute_translations::Model>,
) -> catalogv1::ProductAttribute {
    let available_languages: Vec<String> = translations
        .iter()
        .map(|t| t.language_code.clone())
        .collect();
    catalogv1::ProductAttribute {
        id: Some(r.id),
        sort_order: r.sort_order.map(|v| v as u32),
        translations: translations
            .into_iter()
            .map(attr_translation_proto)
            .collect(),
        available_languages,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

async fn attr_translations_of(
    db: &sea_orm::DatabaseConnection,
    id: i64,
) -> Result<Vec<product_attribute_translations::Model>, Status> {
    repo::translations_of(db, id).await
}

async fn attribute_of(db: &sea_orm::DatabaseConnection, id: i64) -> Result<catalogv1::ProductAttribute, Status> {
    let row = repo::attribute_by_id(db, id).await?;
    let translations = attr_translations_of(db, row.id).await?;
    Ok(attribute_proto(row, translations))
}

/// The main-table write out of the proto `ProductAttribute` data payload (the
/// translation rows ride their own methods — the reference's service
/// keeps the same split).
fn attribute_active(
    data: catalogv1::ProductAttribute,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> product_attributes::ActiveModel {
    product_attributes::ActiveModel {
        sort_order: data.sort_order.map(|v| Set(Some(v as i32))).unwrap_or(Set(None::<i32>)),
        created_at: Set(Some(now.into())),
        ..Default::default()
    }
}

pub struct ProductAttributeService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl catalogv1::product_attribute_service_server::ProductAttributeService for ProductAttributeService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::ListProductAttributeResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            product_attributes::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        let mut items = Vec::with_capacity(rows.len());
        for r in rows {
            let translations = attr_translations_of(&self.state.db, r.id).await?;
            items.push(attribute_proto(r, translations));
        }
        Ok(Response::new(catalogv1::ListProductAttributeResponse {
            items,
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::CountProductAttributeResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            product_attributes::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::CountProductAttributeResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<catalogv1::GetProductAttributeRequest>,
    ) -> Result<Response<catalogv1::ProductAttribute>, Status> {
        let req = request.into_inner();
        let row = match req.query_by {
            Some(catalogv1::get_product_attribute_request::QueryBy::Id(id)) => {
                attribute_of(&self.state.db, id).await?
            }
            None => return Err(bad("query_by required")),
        };
        Ok(Response::new(row))
    }

    async fn create(
        &self,
        request: Request<catalogv1::CreateProductAttributeRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let mut data = req.data.ok_or_else(|| bad("data required"))?;
        let now = store::now();
        let translations = std::mem::take(&mut data.translations);
        let active = attribute_active(data, now);
        let row = repo::insert_attribute(&self.state.db, active).await?;
        // The payload's translation rows land with the brand (the
        // reference's create carries the same dual write).
        for t in translations {
            let ta = attr_translation_active(t, row.id, now)?;
            repo::insert_translation(&self.state.db, ta).await?;
        }
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn batch_create(
        &self,
        request: Request<catalogv1::BatchCreateProductAttributesRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let now = store::now();
        let mut created = Vec::with_capacity(req.items.len());
        for data in req.items {
            let active = attribute_active(data, now);
            let row = repo::insert_attribute(&self.state.db, active).await?;
            created.push(row.id);
        }
        let _ = created;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<catalogv1::UpdateProductAttributeRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = repo::attribute_by_id(&self.state.db, req.id).await?;
        let mut active: product_attributes::ActiveModel = existing.into();
        if let Some(v) = data.sort_order {
            active.sort_order = Set(Some(v as i32));
        }
        active.updated_at = Set(Some(store::now().into()));
        repo::update_attribute(&self.state.db, active).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<catalogv1::DeleteProductAttributeRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::delete_product_attribute_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        repo::delete_attribute(&self.state.db, id).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn translation_exists(
        &self,
        request: Request<catalogv1::ProductAttributeTranslationExistsRequest>,
    ) -> Result<Response<catalogv1::ProductAttributeTranslationExistsResponse>, Status> {
        let req = request.into_inner();
        let row =
            repo::translation_of(&self.state.db, req.attribute_id, &req.language_code).await?;
        Ok(Response::new(catalogv1::ProductAttributeTranslationExistsResponse {
            exists: row.is_some(),
        }))
    }

    async fn get_translation(
        &self,
        request: Request<catalogv1::GetProductAttributeRequest>,
    ) -> Result<Response<catalogv1::ProductAttributeTranslation>, Status> {
        let req = request.into_inner();
        let id = match req.query_by {
            Some(catalogv1::get_product_attribute_request::QueryBy::Id(id)) => id,
            _ => return Err(bad("query_by.id required")),
        };
        let locale = req
            .locale
            .filter(|l| !l.is_empty())
            .ok_or_else(|| bad("locale required"))?;
        let row = repo::translation_of(&self.state.db, id, &locale)
            .await?
            .ok_or_else(|| tonic::Status::not_found("brand translation not found"))?;
        Ok(Response::new(attr_translation_proto(row)))
    }

    async fn create_translation(
        &self,
        request: Request<catalogv1::CreateProductAttributeTranslationRequest>,
    ) -> Result<Response<catalogv1::ProductAttributeTranslation>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let attribute_id = data.attribute_id.ok_or_else(|| bad("data.attributeId required"))?;
        let active = attr_translation_active(data, attribute_id, store::now())?;
        let row = repo::insert_translation(&self.state.db, active).await?;
        Ok(Response::new(attr_translation_proto(row)))
    }

    async fn update_translation(
        &self,
        request: Request<catalogv1::UpdateProductAttributeTranslationRequest>,
    ) -> Result<Response<catalogv1::ProductAttributeTranslation>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = product_attribute_translations::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("translation not found"))?;
        let mut active: product_attribute_translations::ActiveModel = existing.into();
        if let Some(v) = data.name {
            active.name = Set(v);
        }
        active.updated_at = Set(Some(store::now().into()));
        let row = repo::update_translation(&self.state.db, active).await?;
        Ok(Response::new(attr_translation_proto(row)))
    }

    async fn delete_translation(
        &self,
        request: Request<catalogv1::DeleteProductAttributeTranslationRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let id = match req.query_by {
            Some(catalogv1::delete_product_attribute_translation_request::QueryBy::Id(id)) => id,
            Some(catalogv1::delete_product_attribute_translation_request::QueryBy::Identifier(
                identifier,
            )) => {
                let row = repo::translation_of(
                    &self.state.db,
                    identifier.attribute_id,
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
fn attr_translation_active(
    data: catalogv1::ProductAttributeTranslation,
    attribute_id: i64,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Result<product_attribute_translations::ActiveModel, Status> {
    let language_code = data
        .language_code
        .filter(|l| !l.is_empty())
        .ok_or_else(|| bad("data.languageCode required"))?;
    let name = data
        .name
        .filter(|n| !n.is_empty())
        .ok_or_else(|| bad("data.name required"))?;
    Ok(product_attribute_translations::ActiveModel {
        attribute_id: Set(attribute_id),
        language_code: Set(language_code),
        name: Set(name),
        created_at: Set(Some(now.into())),
        ..Default::default()
    })
}

// db_status stays referenced for the repo layer's error mapping.
#[allow(dead_code)]
fn _keep(e: sea_orm::DbErr) -> Status {
    db_status(e)
}
