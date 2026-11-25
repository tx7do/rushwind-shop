//! The domain gRPC service implementations. A service absent from the
//! served list (or a method left to its generated default) answers
//! Unimplemented — the reference's servers register every module the
//! same way and land their logic module by module.

pub mod authentication_service;
pub mod tenant_service;
pub mod org_unit_service;
pub mod position_service;
pub mod constants;
pub mod context;
pub mod permission_sync;
pub mod redis_cache_service;
pub mod role_service;
pub mod menu_service;
pub mod api_service;
pub mod permission_group_service;
pub mod permission_service;
pub mod policy_evaluation_log_service;
pub mod dict_type_service;
pub mod dict_entry_service;
pub mod language_service;
pub mod audit_service;
pub mod brand_service;
pub mod cart_service;
pub mod comment_service;
pub mod coupon_service;
pub mod category_service;
pub mod groupon_service;
pub mod identity_service;
pub mod invoice_service;
pub mod internal_message_service;
pub mod interaction_service;
pub mod notification_service;
pub mod monitor_alert_service;
pub mod online_session_service;
pub mod order_service;
pub mod payment_service;
pub mod product_attribute_service;
pub mod product_attribute_value_service;
pub mod product_service;
pub mod server_monitor_service;
pub mod promotion_service;
pub mod stock_service;
pub mod shipping_service;
pub mod tax_service;
pub mod task_service;
pub mod wishlist_service;
pub mod sku_service;
