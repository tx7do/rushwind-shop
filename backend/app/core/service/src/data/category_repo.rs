//! The `category` repositories (category_repo.go / category_translation_repo.go).

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::state::{not_found_status, StatusResult};
use store::entities::{categories, category_translations};

pub async fn category_by_id(db: &DatabaseConnection, id: i64) -> StatusResult<categories::Model> {
    categories::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(crate::db_status)?
        .ok_or_else(|| not_found_status("category"))
}

/// The `code` query — the brands table carries no code column; the
/// unique code lives on the translation's slug (the proto doc: "slug 或
/// 其他唯一标识符"). Resolve through the translation table.
pub async fn category_by_code(
    db: &DatabaseConnection,
    code: &str,
) -> StatusResult<categories::Model> {
    let t = category_translations::Entity::find()
        .filter(category_translations::Column::Slug.eq(code))
        .one(db)
        .await
        .map_err(crate::db_status)?
        .ok_or_else(|| not_found_status("category"))?;
    category_by_id(db, t.category_id).await
}

pub async fn insert_category(
    db: &DatabaseConnection,
    a: categories::ActiveModel,
) -> StatusResult<categories::Model> {
    a.insert(db).await.map_err(crate::db_status)
}

pub async fn update_category(
    db: &DatabaseConnection,
    a: categories::ActiveModel,
) -> StatusResult<categories::Model> {
    a.update(db).await.map_err(crate::db_status)
}

pub async fn delete_category(db: &DatabaseConnection, id: i64) -> StatusResult<()> {
    categories::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(crate::db_status)?;
    Ok(())
}

pub async fn translation_by_id(
    db: &DatabaseConnection,
    id: i64,
) -> StatusResult<category_translations::Model> {
    category_translations::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(crate::db_status)?
        .ok_or_else(|| not_found_status("category translation"))
}

pub async fn translations_of(
    db: &DatabaseConnection,
    category_id: i64,
) -> StatusResult<Vec<category_translations::Model>> {
    category_translations::Entity::find()
        .filter(category_translations::Column::CategoryId.eq(category_id))
        .all(db)
        .await
        .map_err(crate::db_status)
}

pub async fn translation_of(
    db: &DatabaseConnection,
    category_id: i64,
    language_code: &str,
) -> StatusResult<Option<category_translations::Model>> {
    category_translations::Entity::find()
        .filter(category_translations::Column::CategoryId.eq(category_id))
        .filter(category_translations::Column::LanguageCode.eq(language_code))
        .one(db)
        .await
        .map_err(crate::db_status)
}

pub async fn insert_translation(
    db: &DatabaseConnection,
    a: category_translations::ActiveModel,
) -> StatusResult<category_translations::Model> {
    a.insert(db).await.map_err(crate::db_status)
}

pub async fn update_translation(
    db: &DatabaseConnection,
    a: category_translations::ActiveModel,
) -> StatusResult<category_translations::Model> {
    a.update(db).await.map_err(crate::db_status)
}

pub async fn delete_translation(db: &DatabaseConnection, id: i64) -> StatusResult<()> {
    category_translations::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(crate::db_status)?;
    Ok(())
}
