//! The MFA factor repo — the `sys_user_mfa_factors` face (the
//! reference's user_mfa_factor_repo.go): the per-user listing, the
//! enabled-TOTP lookups, the create with the secret's encrypt-at-rest,
//! and the ownership-checked deletes. This round carries TOTP only —
//! the other methods stay enum values the service refuses.

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::Status;

use crate::state::{db_status, StatusResult};
use store::crypto;
use store::entities::sys_user_mfa_factors as entity;

/// The enrolled factor metadata the management faces render — never
/// the secret.
pub struct EnrolledFactorInfo {
    pub id: i64,
    pub method: String,
    pub display_name: String,
    pub enabled: bool,
    pub created_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub last_used_at: Option<chrono::DateTime<chrono::FixedOffset>>,
}

fn info_of(r: entity::Model) -> EnrolledFactorInfo {
    EnrolledFactorInfo {
        id: r.id,
        method: r.method.unwrap_or_else(|| "TOTP".to_string()),
        display_name: r.display_name.unwrap_or_default(),
        enabled: r.status.as_deref() == Some("ENABLED"),
        created_at: r.created_at,
        last_used_at: r.last_used_at,
    }
}

/// Whether an ENABLED TOTP factor already rides the user (the
/// re-enroll pre-check; the unique index is the hard backstop).
pub async fn has_enabled_totp(
    db: &sea_orm::DatabaseConnection,
    tenant_id: i64,
    user_id: i64,
) -> StatusResult<bool> {
    let found = entity::Entity::find()
        .filter(entity::Column::TenantId.eq(tenant_id))
        .filter(entity::Column::UserId.eq(user_id))
        .filter(entity::Column::Method.eq("TOTP"))
        .filter(entity::Column::Status.eq("ENABLED"))
        .one(db)
        .await
        .map_err(db_status)?;
    Ok(found.is_some())
}

/// The user's ENABLED TOTP factor with its secret decrypted back to
/// the base32 text (the challenge verify path's lookup — the caller
/// decodes the text into the raw key bytes). `None` when the factor is
/// gone or unreadable — the caller voids the challenge.
pub async fn find_enabled_totp_secret(
    db: &sea_orm::DatabaseConnection,
    tenant_id: i64,
    user_id: i64,
) -> Option<(i64, String)> {
    let row = entity::Entity::find()
        .filter(entity::Column::TenantId.eq(tenant_id))
        .filter(entity::Column::UserId.eq(user_id))
        .filter(entity::Column::Method.eq("TOTP"))
        .filter(entity::Column::Status.eq("ENABLED"))
        .one(db)
        .await
        .ok()??;
    let stored = row.secret_hash?;
    use base64::Engine as _;
    let cipher = base64::engine::general_purpose::STANDARD
        .decode(stored)
        .ok()?;
    let plain = crypto::decrypt_aes_cbc(&cipher)?;
    Some((row.id, plain))
}

/// The per-user factor list (the management faces' source — never the
/// secret).
pub async fn list_by_user(
    db: &sea_orm::DatabaseConnection,
    tenant_id: i64,
    user_id: i64,
) -> StatusResult<Vec<EnrolledFactorInfo>> {
    let rows = entity::Entity::find()
        .filter(entity::Column::TenantId.eq(tenant_id))
        .filter(entity::Column::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(db_status)?;
    Ok(rows.into_iter().map(info_of).collect())
}

/// Creates the ENABLED TOTP factor, the secret encrypted at rest
/// (base64(AES-CBC)), and returns its id.
pub async fn create_totp_factor(
    db: &sea_orm::DatabaseConnection,
    tenant_id: i64,
    user_id: i64,
    plain_secret: &str,
    display_name: &str,
) -> StatusResult<i64> {
    use base64::Engine as _;
    let cipher = crypto::encrypt_aes_cbc(plain_secret.as_bytes())
        .ok_or_else(|| Status::internal("encrypt mfa secret failed"))?;
    let enc_secret = base64::engine::general_purpose::STANDARD.encode(cipher);
    let row = entity::ActiveModel {
        tenant_id: Set(Some(tenant_id)),
        user_id: Set(Some(user_id)),
        method: Set(Some("TOTP".to_string())),
        secret_hash: Set(Some(enc_secret)),
        display_name: Set(Some(display_name.to_string())),
        status: Set(Some("ENABLED".to_string())),
        created_at: Set(Some(store::now())),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(db_status)?;
    Ok(row.id)
}

/// Deletes one factor under the (tenant, user) ownership guard —
/// `false` when nothing matched (the not-found answer).
pub async fn delete_for_user(
    db: &sea_orm::DatabaseConnection,
    tenant_id: i64,
    user_id: i64,
    factor_id: i64,
) -> StatusResult<bool> {
    let res = entity::Entity::delete_many()
        .filter(entity::Column::Id.eq(factor_id))
        .filter(entity::Column::TenantId.eq(tenant_id))
        .filter(entity::Column::UserId.eq(user_id))
        .exec(db)
        .await
        .map_err(db_status)?;
    Ok(res.rows_affected > 0)
}

/// Deletes every factor of one (user, method) — the per-method disable
/// face and the admin rescue path.
pub async fn delete_all_by_user_method(
    db: &sea_orm::DatabaseConnection,
    tenant_id: i64,
    user_id: i64,
    method: &str,
) -> StatusResult<u64> {
    let res = entity::Entity::delete_many()
        .filter(entity::Column::TenantId.eq(tenant_id))
        .filter(entity::Column::UserId.eq(user_id))
        .filter(entity::Column::Method.eq(method))
        .exec(db)
        .await
        .map_err(db_status)?;
    Ok(res.rows_affected)
}

/// Locates a factor by id (the admin rescue path's target resolve).
pub async fn get_factor_by_id(
    db: &sea_orm::DatabaseConnection,
    factor_id: i64,
) -> StatusResult<Option<(i64, i64)>> {
    let row = entity::Entity::find_by_id(factor_id)
        .one(db)
        .await
        .map_err(db_status)?;
    Ok(row.and_then(|r| Some((r.tenant_id?, r.user_id?))))
}

/// The first factor row of one user regardless of tenant (the admin
/// rescue path's tenant discovery).
pub async fn find_first_by_user(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    method: &str,
) -> StatusResult<Option<(i64, i64)>> {
    let row = entity::Entity::find()
        .filter(entity::Column::UserId.eq(user_id))
        .filter(entity::Column::Method.eq(method))
        .one(db)
        .await
        .map_err(db_status)?;
    Ok(row.and_then(|r| Some((r.tenant_id?, r.user_id?))))
}

/// The best-effort last-used stamp (a failure never blocks the login).
pub async fn update_last_used(
    db: &sea_orm::DatabaseConnection,
    tenant_id: i64,
    user_id: i64,
    factor_id: i64,
) {
    if let Ok(Some(row)) = entity::Entity::find_by_id(factor_id).one(db).await {
        if row.tenant_id == Some(tenant_id) && row.user_id == Some(user_id) {
            let mut active: entity::ActiveModel = row.into();
            active.last_used_at = Set(Some(store::now()));
            active.updated_at = Set(Some(store::now()));
            let _ = active.update(db).await;
        }
    }
}
