//! The `cart` repositories (cart_repo.go / cart_item_repo.go): the
//! user's cart row and its item rows.

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::state::{not_found_status, StatusResult};
use store::entities::{cart_items, carts};

pub async fn cart_by_id(db: &DatabaseConnection, id: i64) -> StatusResult<carts::Model> {
    carts::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(crate::db_status)?
        .ok_or_else(|| not_found_status("cart"))
}

/// The user's own cart (one live row per user — the lookup the
/// storefront's add-to-cart face resolves first).
pub async fn cart_of_user(db: &DatabaseConnection, user_id: i64) -> StatusResult<Option<carts::Model>> {
    carts::Entity::find()
        .filter(carts::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(crate::db_status)
}

pub async fn insert_cart(db: &DatabaseConnection, a: carts::ActiveModel) -> StatusResult<carts::Model> {
    a.insert(db).await.map_err(crate::db_status)
}

pub async fn update_cart(db: &DatabaseConnection, a: carts::ActiveModel) -> StatusResult<carts::Model> {
    a.update(db).await.map_err(crate::db_status)
}

pub async fn delete_cart(db: &DatabaseConnection, id: i64) -> StatusResult<()> {
    carts::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(crate::db_status)?;
    Ok(())
}

pub async fn cart_item_by_id(db: &DatabaseConnection, id: i64) -> StatusResult<cart_items::Model> {
    cart_items::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(crate::db_status)?
        .ok_or_else(|| not_found_status("cart item"))
}

pub async fn items_of_cart(db: &DatabaseConnection, cart_id: i64) -> StatusResult<Vec<cart_items::Model>> {
    cart_items::Entity::find()
        .filter(cart_items::Column::CartId.eq(cart_id))
        .all(db)
        .await
        .map_err(crate::db_status)
}

pub async fn insert_item(
    db: &DatabaseConnection,
    a: cart_items::ActiveModel,
) -> StatusResult<cart_items::Model> {
    a.insert(db).await.map_err(crate::db_status)
}

pub async fn update_item(
    db: &DatabaseConnection,
    a: cart_items::ActiveModel,
) -> StatusResult<cart_items::Model> {
    a.update(db).await.map_err(crate::db_status)
}

pub async fn delete_item(db: &DatabaseConnection, id: i64) -> StatusResult<()> {
    cart_items::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(crate::db_status)?;
    Ok(())
}
