//! The product-attribute repositories (product_attribute_repo.go /
//! product_attribute_value_repo.go and the translation tables).

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::state::{not_found_status, StatusResult};
use store::entities::{
    product_attribute_translations, product_attribute_value_translations, product_attribute_values,
    product_attributes,
};

pub async fn attribute_by_id(
    db: &DatabaseConnection,
    id: i64,
) -> StatusResult<product_attributes::Model> {
    product_attributes::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(crate::db_status)?
        .ok_or_else(|| not_found_status("product attribute"))
}

pub async fn insert_attribute(
    db: &DatabaseConnection,
    a: product_attributes::ActiveModel,
) -> StatusResult<product_attributes::Model> {
    a.insert(db).await.map_err(crate::db_status)
}

pub async fn update_attribute(
    db: &DatabaseConnection,
    a: product_attributes::ActiveModel,
) -> StatusResult<product_attributes::Model> {
    a.update(db).await.map_err(crate::db_status)
}

pub async fn delete_attribute(db: &DatabaseConnection, id: i64) -> StatusResult<()> {
    product_attributes::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(crate::db_status)?;
    Ok(())
}

pub async fn translations_of(
    db: &DatabaseConnection,
    attribute_id: i64,
) -> StatusResult<Vec<product_attribute_translations::Model>> {
    product_attribute_translations::Entity::find()
        .filter(product_attribute_translations::Column::AttributeId.eq(attribute_id))
        .all(db)
        .await
        .map_err(crate::db_status)
}

pub async fn translation_of(
    db: &DatabaseConnection,
    attribute_id: i64,
    language_code: &str,
) -> StatusResult<Option<product_attribute_translations::Model>> {
    product_attribute_translations::Entity::find()
        .filter(product_attribute_translations::Column::AttributeId.eq(attribute_id))
        .filter(product_attribute_translations::Column::LanguageCode.eq(language_code))
        .one(db)
        .await
        .map_err(crate::db_status)
}

pub async fn insert_translation(
    db: &DatabaseConnection,
    a: product_attribute_translations::ActiveModel,
) -> StatusResult<product_attribute_translations::Model> {
    a.insert(db).await.map_err(crate::db_status)
}

pub async fn update_translation(
    db: &DatabaseConnection,
    a: product_attribute_translations::ActiveModel,
) -> StatusResult<product_attribute_translations::Model> {
    a.update(db).await.map_err(crate::db_status)
}

pub async fn delete_translation(db: &DatabaseConnection, id: i64) -> StatusResult<()> {
    product_attribute_translations::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(crate::db_status)?;
    Ok(())
}

// ── values ──────────────────────────────────────────────────────────

pub async fn value_by_id(
    db: &DatabaseConnection,
    id: i64,
) -> StatusResult<product_attribute_values::Model> {
    use store::entities::product_attribute_values;
    product_attribute_values::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(crate::db_status)?
        .ok_or_else(|| not_found_status("product attribute value"))
}

pub async fn values_of(
    db: &DatabaseConnection,
    attribute_id: i64,
) -> StatusResult<Vec<product_attribute_values::Model>> {
    use store::entities::product_attribute_values;
    product_attribute_values::Entity::find()
        .filter(product_attribute_values::Column::AttributeId.eq(attribute_id))
        .all(db)
        .await
        .map_err(crate::db_status)
}

pub async fn insert_value(
    db: &DatabaseConnection,
    a: store::entities::product_attribute_values::ActiveModel,
) -> StatusResult<store::entities::product_attribute_values::Model> {
    a.insert(db).await.map_err(crate::db_status)
}

pub async fn update_value(
    db: &DatabaseConnection,
    a: store::entities::product_attribute_values::ActiveModel,
) -> StatusResult<store::entities::product_attribute_values::Model> {
    a.update(db).await.map_err(crate::db_status)
}

pub async fn delete_value(db: &DatabaseConnection, id: i64) -> StatusResult<()> {
    use store::entities::product_attribute_values;
    product_attribute_values::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(crate::db_status)?;
    Ok(())
}

pub async fn value_translations_of(
    db: &DatabaseConnection,
    value_id: i64,
) -> StatusResult<Vec<product_attribute_value_translations::Model>> {
    product_attribute_value_translations::Entity::find()
        .filter(product_attribute_value_translations::Column::ValueId.eq(value_id))
        .all(db)
        .await
        .map_err(crate::db_status)
}

pub async fn value_translation_of(
    db: &DatabaseConnection,
    value_id: i64,
    language_code: &str,
) -> StatusResult<Option<product_attribute_value_translations::Model>> {
    product_attribute_value_translations::Entity::find()
        .filter(product_attribute_value_translations::Column::ValueId.eq(value_id))
        .filter(product_attribute_value_translations::Column::LanguageCode.eq(language_code))
        .one(db)
        .await
        .map_err(crate::db_status)
}

pub async fn insert_value_translation(
    db: &DatabaseConnection,
    a: product_attribute_value_translations::ActiveModel,
) -> StatusResult<product_attribute_value_translations::Model> {
    a.insert(db).await.map_err(crate::db_status)
}

pub async fn update_value_translation(
    db: &DatabaseConnection,
    a: product_attribute_value_translations::ActiveModel,
) -> StatusResult<product_attribute_value_translations::Model> {
    a.update(db).await.map_err(crate::db_status)
}

pub async fn delete_value_translation(db: &DatabaseConnection, id: i64) -> StatusResult<()> {
    product_attribute_value_translations::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(crate::db_status)?;
    Ok(())
}
