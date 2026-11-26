//! The product-attribute-value service — the option values under an
//! attribute, each with the translation sub-table (the reference's
//! product_attribute_value_service.go).

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use tonic::{Request, Response, Status};

use crate::data::attr_repo as repo;
use crate::state::{bad, db_status, AppState};
use store::entities::{product_attribute_value_translations, product_attribute_values};
use store::paging::fetch_paged;

use shop_proto::proto::catalog::service::v1 as catalogv1;
use shop_proto::proto::pagination::PagingRequest;

fn value_translation_proto(
    r: product_attribute_value_translations::Model,
) -> catalogv1::ProductAttributeValueTranslation {
    catalogv1::ProductAttributeValueTranslation {
        id: Some(r.id),
        value_id: Some(r.value_id),
        language_code: Some(r.language_code.clone()),
        display_name: Some(r.display_name.clone()),
        ..Default::default()
    }
}

async fn value_with_translations(
    db: &sea_orm::DatabaseConnection,
    id: i64,
) -> Result<
    (
        product_attribute_values::Model,
        Vec<product_attribute_value_translations::Model>,
    ),
    Status,
> {
    let row = product_attribute_values::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(db_status)?
        .ok_or_else(|| Status::not_found("attribute value not found"))?;
    let translations = repo::value_translations_of(db, row.id).await?;
    Ok((row, translations))
}

fn value_proto(
    r: product_attribute_values::Model,
    translations: Vec<product_attribute_value_translations::Model>,
) -> catalogv1::ProductAttributeValue {
    let available_languages: Vec<String> = translations
        .iter()
        .map(|t| t.language_code.clone())
        .collect();
    catalogv1::ProductAttributeValue {
        id: Some(r.id),
        attribute_id: Some(r.attribute_id),
        sort_order: r.sort_order.map(|v| v as u32),
        translations: translations
            .into_iter()
            .map(value_translation_proto)
            .collect(),
        available_languages,
        ..Default::default()
    }
}

pub struct ProductAttributeValueService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl catalogv1::product_attribute_value_service_server::ProductAttributeValueService
    for ProductAttributeValueService
{
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::ListProductAttributeValueResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            product_attribute_values::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        let mut items = Vec::with_capacity(rows.len());
        for r in rows {
            let translations = repo::value_translations_of(&self.state.db, r.id).await?;
            items.push(value_proto(r, translations));
        }
        Ok(Response::new(
            catalogv1::ListProductAttributeValueResponse {
                items,
                total: total as i64,
            },
        ))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::CountProductAttributeValueResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            product_attribute_values::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(
            catalogv1::CountProductAttributeValueResponse {
                count: total as i64,
            },
        ))
    }

    async fn get(
        &self,
        request: Request<catalogv1::GetProductAttributeValueRequest>,
    ) -> Result<Response<catalogv1::ProductAttributeValue>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::get_product_attribute_value_request::QueryBy::Id(id)) = req.query_by
        else {
            return Err(bad("query_by required"));
        };
        let (row, translations) = value_with_translations(&self.state.db, id).await?;
        Ok(Response::new(value_proto(row, translations)))
    }

    async fn create(
        &self,
        request: Request<catalogv1::CreateProductAttributeValueRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let attribute_id = data
            .attribute_id
            .ok_or_else(|| bad("data.attributeId required"))?;
        let active = product_attribute_values::ActiveModel {
            attribute_id: Set(attribute_id),
            sort_order: Set(data.sort_order.map(|v| v as i32)),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        let row = active.insert(&self.state.db).await.map_err(db_status)?;
        for t in &data.translations {
            let active = product_attribute_value_translations::ActiveModel {
                value_id: Set(row.id),
                language_code: Set(t.language_code.clone().unwrap_or_default()),
                display_name: Set(t.display_name.clone().unwrap_or_default()),
                created_at: Set(Some(store::now())),
                ..Default::default()
            };
            repo::insert_value_translation(&self.state.db, active).await?;
        }
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn batch_create(
        &self,
        request: Request<catalogv1::BatchCreateProductAttributeValuesRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        for data in req.items {
            let Some(attribute_id) = data.attribute_id else {
                return Err(bad("data.attributeId required"));
            };
            let active = product_attribute_values::ActiveModel {
                attribute_id: Set(attribute_id),
                sort_order: Set(data.sort_order.map(|v| v as i32)),
                created_at: Set(Some(store::now())),
                ..Default::default()
            };
            active.insert(&self.state.db).await.map_err(db_status)?;
        }
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<catalogv1::UpdateProductAttributeValueRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let (row, _) = value_with_translations(&self.state.db, req.id).await?;
        let mut active: product_attribute_values::ActiveModel = row.into();
        if let Some(v) = data.sort_order {
            active.sort_order = Set(Some(v as i32));
        }
        active.updated_at = Set(Some(store::now()));
        repo::update_value(&self.state.db, active).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<catalogv1::DeleteProductAttributeValueRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::delete_product_attribute_value_request::QueryBy::Id(id)) = req.query_by
        else {
            return Err(bad("query_by required"));
        };
        repo::delete_value(&self.state.db, id).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn translation_exists(
        &self,
        request: Request<catalogv1::ProductAttributeValueTranslationExistsRequest>,
    ) -> Result<Response<catalogv1::ProductAttributeValueTranslationExistsResponse>, Status> {
        let req = request.into_inner();
        let row =
            repo::value_translation_of(&self.state.db, req.value_id, &req.language_code).await?;
        Ok(Response::new(
            catalogv1::ProductAttributeValueTranslationExistsResponse {
                exists: row.is_some(),
            },
        ))
    }

    async fn get_translation(
        &self,
        request: Request<catalogv1::GetProductAttributeValueRequest>,
    ) -> Result<Response<catalogv1::ProductAttributeValueTranslation>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::get_product_attribute_value_request::QueryBy::Id(id)) = req.query_by
        else {
            return Err(bad("query_by required"));
        };
        let locale = req
            .locale
            .filter(|l| !l.is_empty())
            .ok_or_else(|| bad("locale required"))?;
        let row = repo::value_translation_of(&self.state.db, id, &locale)
            .await?
            .ok_or_else(|| Status::not_found("translation not found"))?;
        Ok(Response::new(value_translation_proto(row)))
    }

    async fn create_translation(
        &self,
        request: Request<catalogv1::CreateProductAttributeValueTranslationRequest>,
    ) -> Result<Response<catalogv1::ProductAttributeValueTranslation>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let value_id = data.value_id.ok_or_else(|| bad("data.valueId required"))?;
        let active = product_attribute_value_translations::ActiveModel {
            value_id: Set(value_id),
            language_code: Set(data.language_code.clone().unwrap_or_default()),
            display_name: Set(data.display_name.clone().unwrap_or_default()),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        let row = repo::insert_value_translation(&self.state.db, active).await?;
        Ok(Response::new(value_translation_proto(row)))
    }

    async fn update_translation(
        &self,
        request: Request<catalogv1::UpdateProductAttributeValueTranslationRequest>,
    ) -> Result<Response<catalogv1::ProductAttributeValueTranslation>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let locale = data
            .language_code
            .clone()
            .filter(|l| !l.is_empty())
            .ok_or_else(|| bad("data.languageCode required"))?;
        let row = repo::value_translation_of(&self.state.db, req.id, &locale)
            .await?
            .ok_or_else(|| Status::not_found("translation not found"))?;
        let mut active: product_attribute_value_translations::ActiveModel = row.into();
        if let Some(v) = data.display_name.clone() {
            active.display_name = Set(v);
        }
        active.updated_at = Set(Some(store::now()));
        let row = repo::update_value_translation(&self.state.db, active).await?;
        Ok(Response::new(value_translation_proto(row)))
    }

    async fn delete_translation(
        &self,
        request: Request<catalogv1::DeleteProductAttributeValueTranslationRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::delete_product_attribute_value_translation_request::QueryBy::Id(id)) =
            req.query_by
        else {
            return Err(bad("query_by required"));
        };
        repo::delete_value_translation(&self.state.db, id).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}
