//! The login-policy repository — the `sys_login_policies` face (the
//! reference's login_policy_repo.go): the by-id fetch and the
//! insert/update/delete the service rides.

use sea_orm::{DatabaseConnection, EntityTrait};

use crate::state::{db_status, not_found_status, StatusResult};
use store::entities::sys_login_policies;

pub async fn login_policies_by_id(
    db: &DatabaseConnection,
    id: i64,
) -> StatusResult<sys_login_policies::Model> {
    sys_login_policies::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(db_status)?
        .ok_or_else(|| not_found_status("login policies"))
}

pub async fn insert_login_policies(
    db: &DatabaseConnection,
    a: sys_login_policies::ActiveModel,
) -> StatusResult<sys_login_policies::Model> {
    use sea_orm::ActiveModelTrait as _;
    a.insert(db).await.map_err(db_status)
}

pub async fn delete_login_policies(db: &DatabaseConnection, id: i64) -> StatusResult<()> {
    sys_login_policies::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(db_status)?;
    Ok(())
}
