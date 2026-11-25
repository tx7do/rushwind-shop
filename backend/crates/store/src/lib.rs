//! The shared data layer of the Shop deployments: the generated entity
//! tree, the paging/filter select assembly, and the bootstrap migration
//! (golden DDL).
//!
//! The entity modules are hand-shaped to the golden DDL
//! (`backend/sql/schema.sql`) — the Go reference's Ent migration
//! produced the same columns (the demo-data INSERT lists pin the
//! spellings); regenerate the pair together, never let them drift.

pub mod bootstrap;
pub mod entities;
pub mod paging;

use sea_orm::DatabaseConnection;

/// The golden DDL, embedded.
pub const SCHEMA_SQL: &str = include_str!("../../../sql/schema.sql");

// The DB failure mappings and the entity-time helpers live in the
// framework bridge crate; re-exported so the repositories' and
// services' paths stay put.
pub use rushwind_storage_seaorm_support::time::now;
pub use rushwind_storage_seaorm_support::{db_err, internal_error_msg as db_err_internal};

/// The tables exist check — the bootstrap gate. Complete only when the
/// golden DDL's full table set is present (a partial database from an
/// older schema still re-runs the idempotent DDL).
pub async fn tables_present(db: &DatabaseConnection) -> Result<bool, sea_orm::DbErr> {
    use sea_orm::ConnectionTrait as _;
    const TABLES: &[&str] = &[
        "mall_brands",
        "mall_brand_translations",
        "mall_categories",
        "mall_category_translations",
        "mall_carts",
        "mall_cart_items",
        "sys_users",
    ];
    for table in TABLES {
        let row = db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!(
                    "SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace                      WHERE n.nspname='public' AND c.relname='{table}' LIMIT 1"
                ),
            ))
            .await?;
        if row.is_none() {
            return Ok(false);
        }
    }
    Ok(true)
}
