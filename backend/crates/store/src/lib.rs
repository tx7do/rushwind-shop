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
        "mall_products",
        "mall_product_attribute_translations",
        "mall_product_attribute_value_translations",
        "mall_product_attribute_values",
        "mall_product_attributes",
        "mall_product_translations",
        "mall_member_tiers",
        "mall_points_ledgers",
        "mall_promotions",
        "mall_seckill_activities",
        "mall_stock_alerts",
        "mall_stock_op_logs",
        "mall_skus",
        "mall_sku_prices",
        "mall_sku_attribute_combinations",
        "mall_orders",
        "mall_order_items",
        "mall_payment_transactions",
        "mall_payment_refunds",
        "mall_payment_channel_configs",
        "sys_api_audit_logs",
        "sys_data_access_audit_logs",
        "sys_login_audit_logs",
        "sys_operation_audit_logs",
        "sys_permission_audit_logs",
        "mall_comments",
        "mall_interaction_likes",
        "mall_invoices",
        "mall_internal_message_categories",
        "mall_internal_message_recipients",
        "mall_internal_messages",
        "mall_interaction_counters",
        "mall_notification_channels",
        "mall_notification_deliveries",
        "mall_notification_rules",
        "mall_notification_templates",
        "mall_coupon_templates",
        "mall_user_coupons",
        "sys_org_units",
        "sys_positions",
        "sys_tasks",
        "mall_tax_rates",
        "sys_tenants",
        "mall_wishlist",
        "mall_groupon_activities",
        "mall_groupon_groups",
        "mall_shipping_addresses",
        "mall_shipping_rates",
        "mall_shipments",
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
