//! GENERATED helper — the admin face's full service mount sequence.
//!
//! The (mount, null) pairs track `shop-proto`'s generated surface for the
//! admin gateway; regenerate after contract growth with the same
//! sort/paste pass over the generated module's function list.
use axum::Router;
use rushwind_http_binding::wire::RouteWire;
use shop_proto::gen_app::{mounts, nulls};

/// Mounts every service of the admin contract face onto the (public,
/// gated) router pair — each with its null placeholder implementation.
///
/// The `wrap` hook is the deployment's per-route tail; the authn gate
/// itself is applied at the router level (`with_authn` around the gated
/// tree), so the default wrap is the identity.
pub fn mount_all(
    wrap: &dyn Fn(axum::routing::MethodRouter, &RouteWire, bool) -> axum::routing::MethodRouter,
) -> (Router, Router) {
    macro_rules! next {
        ($m:ident, $n:ident) => {{
            let (p, g) = mounts::$m(acc.0, acc.1, nulls::$n(), wrap);
            acc = (p, g);
        }};
    }
    let mut acc = (Router::new(), Router::new());
    next!(mount_access_key_service, null_access_key_service);
    next!(mount_admin_portal_service, null_admin_portal_service);
    next!(mount_ai_agent_service, null_ai_agent_service);
    next!(mount_ai_chat_service, null_ai_chat_service);
    next!(mount_ai_content_service, null_ai_content_service);
    next!(mount_ai_conversation_service, null_ai_conversation_service);
    next!(mount_ai_knowledge_base_service, null_ai_knowledge_base_service);
    next!(mount_ai_message_service, null_ai_message_service);
    next!(mount_ai_provider_service, null_ai_provider_service);
    next!(mount_ai_query_service, null_ai_query_service);
    next!(mount_ai_usage_log_service, null_ai_usage_log_service);
    next!(mount_api_audit_log_service, null_api_audit_log_service);
    next!(mount_api_service, null_api_service);
    next!(mount_authentication_service, null_authentication_service);
    next!(mount_brand_service, null_brand_service);
    next!(mount_cart_item_service, null_cart_item_service);
    next!(mount_cart_service, null_cart_service);
    next!(mount_category_service, null_category_service);
    next!(mount_comment_service, null_comment_service);
    next!(mount_config_service, null_config_service);
    next!(mount_coupon_template_service, null_coupon_template_service);
    next!(mount_data_access_audit_log_service, null_data_access_audit_log_service);
    next!(mount_dict_entry_service, null_dict_entry_service);
    next!(mount_dict_type_service, null_dict_type_service);
    next!(mount_file_service, null_file_service);
    next!(mount_file_transfer_service, null_file_transfer_service);
    next!(mount_groupon_activity_service, null_groupon_activity_service);
    next!(mount_interaction_admin_service, null_interaction_admin_service);
    next!(mount_internal_message_category_service, null_internal_message_category_service);
    next!(mount_internal_message_recipient_service, null_internal_message_recipient_service);
    next!(mount_internal_message_service, null_internal_message_service);
    next!(mount_invoice_service, null_invoice_service);
    next!(mount_language_service, null_language_service);
    next!(mount_login_audit_log_service, null_login_audit_log_service);
    next!(mount_login_policy_service, null_login_policy_service);
    next!(mount_member_tier_service, null_member_tier_service);
    next!(mount_menu_service, null_menu_service);
    next!(mount_mfa_service, null_mfa_service);
    next!(mount_monitor_alert_service, null_monitor_alert_service);
    next!(mount_notification_channel_service, null_notification_channel_service);
    next!(mount_notification_preference_service, null_notification_preference_service);
    next!(mount_notification_rule_service, null_notification_rule_service);
    next!(mount_notification_service, null_notification_service);
    next!(mount_notification_template_service, null_notification_template_service);
    next!(mount_online_session_service, null_online_session_service);
    next!(mount_operation_audit_log_service, null_operation_audit_log_service);
    next!(mount_order_item_service, null_order_item_service);
    next!(mount_order_service, null_order_service);
    next!(mount_org_unit_service, null_org_unit_service);
    next!(mount_payment_channel_config_service, null_payment_channel_config_service);
    next!(mount_payment_refund_service, null_payment_refund_service);
    next!(mount_payment_transaction_service, null_payment_transaction_service);
    next!(mount_permission_audit_log_service, null_permission_audit_log_service);
    next!(mount_permission_group_service, null_permission_group_service);
    next!(mount_permission_service, null_permission_service);
    next!(mount_plan_module_service, null_plan_module_service);
    next!(mount_plan_quota_service, null_plan_quota_service);
    next!(mount_plan_service, null_plan_service);
    next!(mount_points_ledger_service, null_points_ledger_service);
    next!(mount_policy_evaluation_log_service, null_policy_evaluation_log_service);
    next!(mount_position_service, null_position_service);
    next!(mount_product_attribute_service, null_product_attribute_service);
    next!(mount_product_attribute_value_service, null_product_attribute_value_service);
    next!(mount_product_service, null_product_service);
    next!(mount_promotion_service, null_promotion_service);
    next!(mount_redis_cache_monitor_service, null_redis_cache_monitor_service);
    next!(mount_role_service, null_role_service);
    next!(mount_script_log_service, null_script_log_service);
    next!(mount_script_service, null_script_service);
    next!(mount_seckill_activity_service, null_seckill_activity_service);
    next!(mount_server_monitor_service, null_server_monitor_service);
    next!(mount_shipment_service, null_shipment_service);
    next!(mount_shipping_rate_service, null_shipping_rate_service);
    next!(mount_sku_attribute_combination_service, null_sku_attribute_combination_service);
    next!(mount_sku_price_service, null_sku_price_service);
    next!(mount_sku_service, null_sku_service);
    next!(mount_stock_alert_service, null_stock_alert_service);
    next!(mount_stock_op_log_service, null_stock_op_log_service);
    next!(mount_task_monitor_service, null_task_monitor_service);
    next!(mount_task_service, null_task_service);
    next!(mount_tax_rate_service, null_tax_rate_service);
    next!(mount_tenant_service, null_tenant_service);
    next!(mount_user_coupon_service, null_user_coupon_service);
    next!(mount_user_profile_service, null_user_profile_service);
    next!(mount_user_service, null_user_service);
    acc
}
