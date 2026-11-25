//! The `brand` repositories (brand_repo.go / brand_translation_repo.go).

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::state::{not_found_status, StatusResult};
use store::entities::{brand_translations, brands};

pub async fn brand_by_id(db: &DatabaseConnection, id: i64) -> StatusResult<brands::Model> {
    brands::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(crate::db_status)?
        .ok_or_else(|| not_found_status("brand"))
}

/// The `code` query — the brands table carries no code column; the
/// unique code lives on the translation's slug (the proto doc: "slug 或
/// 其他唯一标识符"). Resolve through the translation table.
pub async fn brand_by_code(
    db: &DatabaseConnection,
    code: &str,
) -> StatusResult<brands::Model> {
    let t = brand_translations::Entity::find()
        .filter(brand_translations::Column::Slug.eq(code))
        .one(db)
        .await
        .map_err(crate::db_status)?
        .ok_or_else(|| not_found_status("brand"))?;
    brand_by_id(db, t.brand_id).await
}

pub async fn insert_brand(
    db: &DatabaseConnection,
    a: brands::ActiveModel,
) -> StatusResult<brands::Model> {
    a.insert(db).await.map_err(crate::db_status)
}

pub async fn update_brand(
    db: &DatabaseConnection,
    a: brands::ActiveModel,
) -> StatusResult<brands::Model> {
    a.update(db).await.map_err(crate::db_status)
}

pub async fn delete_brand(db: &DatabaseConnection, id: i64) -> StatusResult<()> {
    brands::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(crate::db_status)?;
    Ok(())
}

pub async fn translation_by_id(
    db: &DatabaseConnection,
    id: i64,
) -> StatusResult<brand_translations::Model> {
    brand_translations::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(crate::db_status)?
        .ok_or_else(|| not_found_status("brand translation"))
}

pub async fn translations_of(
    db: &DatabaseConnection,
    brand_id: i64,
) -> StatusResult<Vec<brand_translations::Model>> {
    brand_translations::Entity::find()
        .filter(brand_translations::Column::BrandId.eq(brand_id))
        .all(db)
        .await
        .map_err(crate::db_status)
}

pub async fn translation_of(
    db: &DatabaseConnection,
    brand_id: i64,
    language_code: &str,
) -> StatusResult<Option<brand_translations::Model>> {
    brand_translations::Entity::find()
        .filter(brand_translations::Column::BrandId.eq(brand_id))
        .filter(brand_translations::Column::LanguageCode.eq(language_code))
        .one(db)
        .await
        .map_err(crate::db_status)
}

pub async fn insert_translation(
    db: &DatabaseConnection,
    a: brand_translations::ActiveModel,
) -> StatusResult<brand_translations::Model> {
    a.insert(db).await.map_err(crate::db_status)
}

pub async fn update_translation(
    db: &DatabaseConnection,
    a: brand_translations::ActiveModel,
) -> StatusResult<brand_translations::Model> {
    a.update(db).await.map_err(crate::db_status)
}

pub async fn delete_translation(db: &DatabaseConnection, id: i64) -> StatusResult<()> {
    brand_translations::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(crate::db_status)?;
    Ok(())
}
