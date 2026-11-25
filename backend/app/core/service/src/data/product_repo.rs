//! The `product` repositories (product_repo.go / product_translation_repo.go).

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::state::{not_found_status, StatusResult};
use store::entities::{product_translations, products};

pub async fn product_by_id(db: &DatabaseConnection, id: i64) -> StatusResult<products::Model> {
    products::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(crate::db_status)?
        .ok_or_else(|| not_found_status("product"))
}

/// The `code` query — the brands table carries no code column; the
/// unique code lives on the translation's slug (the proto doc: "slug 或
/// 其他唯一标识符"). Resolve through the translation table.
pub async fn product_by_code(db: &DatabaseConnection, code: &str) -> StatusResult<products::Model> {
    let t = product_translations::Entity::find()
        .filter(product_translations::Column::Slug.eq(code))
        .one(db)
        .await
        .map_err(crate::db_status)?
        .ok_or_else(|| not_found_status("product"))?;
    product_by_id(db, t.product_id).await
}

pub async fn insert_product(
    db: &DatabaseConnection,
    a: products::ActiveModel,
) -> StatusResult<products::Model> {
    a.insert(db).await.map_err(crate::db_status)
}

pub async fn update_product(
    db: &DatabaseConnection,
    a: products::ActiveModel,
) -> StatusResult<products::Model> {
    a.update(db).await.map_err(crate::db_status)
}

pub async fn delete_product(db: &DatabaseConnection, id: i64) -> StatusResult<()> {
    products::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(crate::db_status)?;
    Ok(())
}

pub async fn translation_by_id(
    db: &DatabaseConnection,
    id: i64,
) -> StatusResult<product_translations::Model> {
    product_translations::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(crate::db_status)?
        .ok_or_else(|| not_found_status("product translation"))
}

pub async fn translations_of(
    db: &DatabaseConnection,
    product_id: i64,
) -> StatusResult<Vec<product_translations::Model>> {
    product_translations::Entity::find()
        .filter(product_translations::Column::ProductId.eq(product_id))
        .all(db)
        .await
        .map_err(crate::db_status)
}

pub async fn translation_of(
    db: &DatabaseConnection,
    product_id: i64,
    language_code: &str,
) -> StatusResult<Option<product_translations::Model>> {
    product_translations::Entity::find()
        .filter(product_translations::Column::ProductId.eq(product_id))
        .filter(product_translations::Column::LanguageCode.eq(language_code))
        .one(db)
        .await
        .map_err(crate::db_status)
}

pub async fn insert_translation(
    db: &DatabaseConnection,
    a: product_translations::ActiveModel,
) -> StatusResult<product_translations::Model> {
    a.insert(db).await.map_err(crate::db_status)
}

pub async fn update_translation(
    db: &DatabaseConnection,
    a: product_translations::ActiveModel,
) -> StatusResult<product_translations::Model> {
    a.update(db).await.map_err(crate::db_status)
}

pub async fn delete_translation(db: &DatabaseConnection, id: i64) -> StatusResult<()> {
    product_translations::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(crate::db_status)?;
    Ok(())
}
