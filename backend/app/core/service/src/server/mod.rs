//! The server layer — the gRPC face assembly (the tonic registry),
//! mirroring the reference's `internal/server`. The service modules
//! live in [`crate::service`]; this module owns registration only.

use std::sync::Arc;

use crate::service::{
    self, api_service as api, audit_service as audit, authentication_service as authentication,
    brand_service as brand, cart_service as cart, category_service as category,
    comment_service as comment, coupon_service as coupon,
    file_service as file, file_transfer_service as file_transfer, oss_service as oss, dict_entry_service as dict_entry,
    dict_type_service as dict_type, groupon_service as groupon, identity_service as identity,
    interaction_service as interaction, internal_message_service as internal_message,
    invoice_service as invoice, language_service as language, login_policy_service as login_policy,
    member_tier_service as member_tier,
    menu_service as menu, mfa_service as mfa, monitor_alert_service as monitor,
    notification_service as notification,
    online_session_service as online_session, order_service as order, org_unit_service as org_unit,
    payment_service as payment, plan_service as plan, permission_group_service as perm_group,
    permission_service as perm_svc, points_service as points,
    policy_evaluation_log_service as policy_log, position_service as position,
    product_attribute_service as product_attribute,
    product_attribute_value_service as product_attribute_value, product_service as product,
    promotion_service as promotion, redis_cache_service as redis_cache, role_service as role,
    seckill_service as seckill, server_monitor_service as server_monitor,
    shipping_service as shipping, sku_service as sku, stock_service as stock, task_service as task,
    tax_service as tax, tenant_service as tenant, user_credential_service as user_credential,
    wishlist_service as wishlist,
};
use crate::state::AppState;
use shop_proto::proto::address::service::v1::shipping_address_service_server::ShippingAddressServiceServer;
use shop_proto::proto::audit::service::v1::{
    api_audit_log_service_server::ApiAuditLogServiceServer,
    data_access_audit_log_service_server::DataAccessAuditLogServiceServer,
    login_audit_log_service_server::LoginAuditLogServiceServer,
    operation_audit_log_service_server::OperationAuditLogServiceServer,
    permission_audit_log_service_server::PermissionAuditLogServiceServer,
};
use shop_proto::proto::authentication::service::v1::authentication_service_server::AuthenticationServiceServer;
use shop_proto::proto::identity::service::v1::{
    plan_module_service_server::PlanModuleServiceServer,
    plan_quota_service_server::PlanQuotaServiceServer, plan_service_server::PlanServiceServer,
};
use shop_proto::proto::storage::service::v1::{
    file_service_server::FileServiceServer, file_transfer_service_server::FileTransferServiceServer,
    oss_service_server::OssServiceServer,
};
use shop_proto::proto::authentication::service::v1::login_policy_service_server::LoginPolicyServiceServer;
use shop_proto::proto::authentication::service::v1::mfa_service_server::MfaServiceServer;
use shop_proto::proto::authentication::service::v1::user_credential_service_server::UserCredentialServiceServer;
use shop_proto::proto::cart::service::v1::{
    cart_item_service_server::CartItemServiceServer, cart_service_server::CartServiceServer,
};
use shop_proto::proto::catalog::service::v1::groupon_activity_service_server::GrouponActivityServiceServer;
use shop_proto::proto::catalog::service::v1::{
    brand_service_server::BrandServiceServer, category_service_server::CategoryServiceServer,
    product_service_server::ProductServiceServer,
    sku_attribute_combination_service_server::SkuAttributeCombinationServiceServer,
    sku_price_service_server::SkuPriceServiceServer, sku_service_server::SkuServiceServer,
};
use shop_proto::proto::catalog::service::v1::{
    member_tier_service_server::MemberTierServiceServer,
    points_ledger_service_server::PointsLedgerServiceServer,
    seckill_activity_service_server::SeckillActivityServiceServer,
};
use shop_proto::proto::catalog::service::v1::{
    product_attribute_service_server::ProductAttributeServiceServer,
    product_attribute_value_service_server::ProductAttributeValueServiceServer,
};
use shop_proto::proto::catalog::service::v1::{
    promotion_service_server::PromotionServiceServer,
    stock_alert_service_server::StockAlertServiceServer,
    stock_op_log_service_server::StockOpLogServiceServer,
};
use shop_proto::proto::comment::service::v1::comment_service_server::CommentServiceServer;
use shop_proto::proto::coupon::service::v1::{
    coupon_template_service_server::CouponTemplateServiceServer,
    user_coupon_service_server::UserCouponServiceServer,
};
use shop_proto::proto::dict::service::v1::{
    dict_entry_service_server::DictEntryServiceServer,
    dict_type_service_server::DictTypeServiceServer,
    language_service_server::LanguageServiceServer,
};
use shop_proto::proto::identity::service::v1::{
    org_unit_service_server::OrgUnitServiceServer, position_service_server::PositionServiceServer,
    tenant_service_server::TenantServiceServer,
};
use shop_proto::proto::identity::service::v1::{
    user_profile_service_server::UserProfileServiceServer, user_service_server::UserServiceServer,
};
use shop_proto::proto::interaction::service::v1::{
    interaction_admin_service_server::InteractionAdminServiceServer,
    interaction_service_server::InteractionServiceServer,
};
use shop_proto::proto::internal_message::service::v1::{
    internal_message_category_service_server::InternalMessageCategoryServiceServer,
    internal_message_recipient_service_server::InternalMessageRecipientServiceServer,
    internal_message_service_server::InternalMessageServiceServer,
};
use shop_proto::proto::invoice::service::v1::invoice_service_server::InvoiceServiceServer;
use shop_proto::proto::monitor_alert::service::v1::monitor_alert_service_server::MonitorAlertServiceServer;
use shop_proto::proto::notification::service::v1::{
    notification_channel_service_server::NotificationChannelServiceServer,
    notification_preference_service_server::NotificationPreferenceServiceServer,
    notification_rule_service_server::NotificationRuleServiceServer,
    notification_service_server::NotificationServiceServer,
    notification_template_service_server::NotificationTemplateServiceServer,
};
use shop_proto::proto::online_session::service::v1::online_session_service_server::OnlineSessionServiceServer;
use shop_proto::proto::order::service::v1::{
    order_item_service_server::OrderItemServiceServer, order_service_server::OrderServiceServer,
};
use shop_proto::proto::payment::service::v1::{
    payment_channel_config_service_server::PaymentChannelConfigServiceServer,
    payment_refund_service_server::PaymentRefundServiceServer,
    payment_transaction_service_server::PaymentTransactionServiceServer,
};
use shop_proto::proto::permission::service::v1::{
    api_service_server::ApiServiceServer, menu_service_server::MenuServiceServer,
    permission_group_service_server::PermissionGroupServiceServer,
    permission_service_server::PermissionServiceServer,
    policy_evaluation_log_service_server::PolicyEvaluationLogServiceServer,
    role_service_server::RoleServiceServer,
};
use shop_proto::proto::redis_cache::service::v1::redis_cache_monitor_service_server::RedisCacheMonitorServiceServer;
use shop_proto::proto::server_monitor::service::v1::server_monitor_service_server::ServerMonitorServiceServer;
use shop_proto::proto::shipping::service::v1::{
    shipment_service_server::ShipmentServiceServer,
    shipping_rate_service_server::ShippingRateServiceServer,
};
use shop_proto::proto::task::service::v1::task_service_server::TaskServiceServer;
use shop_proto::proto::tax::service::v1::tax_rate_service_server::TaxRateServiceServer;
use shop_proto::proto::wishlist::service::v1::wishlist_service_server::WishlistServiceServer;

/// The served domain services — one `add_service` per landed module;
/// everything still absent answers Unimplemented by construction (the
/// generated stubs), matching the reference's module-by-module landing.
pub fn registry(state: Arc<AppState>) -> tonic::service::Routes {
    tonic::service::Routes::new(AuthenticationServiceServer::new(
        authentication::AuthenticationService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(UserCredentialServiceServer::new(
        user_credential::UserCredentialServiceImpl {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(MfaServiceServer::new(mfa::MfaService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(LoginPolicyServiceServer::new(login_policy::LoginPolicyService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(FileServiceServer::new(file::FileService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(OssServiceServer::new(oss::OssService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(FileTransferServiceServer::new(file_transfer::FileTransferService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(PlanServiceServer::new(plan::PlanService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(PlanQuotaServiceServer::new(plan::PlanQuotaService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(PlanModuleServiceServer::new(plan::PlanModuleService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(BrandServiceServer::new(brand::BrandService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(CategoryServiceServer::new(category::CategoryService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(CartServiceServer::new(cart::CartService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(CartItemServiceServer::new(cart::CartItemService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(ProductAttributeServiceServer::new(
        product_attribute::ProductAttributeService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(ProductAttributeValueServiceServer::new(
        product_attribute_value::ProductAttributeValueService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(ProductServiceServer::new(product::ProductService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(SkuServiceServer::new(sku::SkuService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(SkuPriceServiceServer::new(sku::SkuPriceService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(SkuAttributeCombinationServiceServer::new(
        sku::SkuAttributeCombinationService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(OrderServiceServer::new(order::OrderService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(OrderItemServiceServer::new(order::OrderItemService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(CouponTemplateServiceServer::new(
        coupon::CouponTemplateService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(UserCouponServiceServer::new(coupon::UserCouponService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(UserServiceServer::new(identity::UserService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(UserProfileServiceServer::new(
        identity::UserProfileService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(ShippingAddressServiceServer::new(
        identity::ShippingAddressService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(GrouponActivityServiceServer::new(
        groupon::GrouponActivityService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(PaymentTransactionServiceServer::new(
        payment::PaymentTransactionService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(PaymentRefundServiceServer::new(
        payment::PaymentRefundService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(PaymentChannelConfigServiceServer::new(
        payment::PaymentChannelConfigService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(ShippingRateServiceServer::new(
        shipping::ShippingRateService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(ShipmentServiceServer::new(shipping::ShipmentService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(WishlistServiceServer::new(wishlist::WishlistService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(CommentServiceServer::new(comment::CommentService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(TaskServiceServer::new(task::TaskService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(TaxRateServiceServer::new(tax::TaxRateService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(PromotionServiceServer::new(promotion::PromotionService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(SeckillActivityServiceServer::new(
        seckill::SeckillActivityService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(PointsLedgerServiceServer::new(
        points::PointsLedgerService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(MemberTierServiceServer::new(
        member_tier::MemberTierService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(StockAlertServiceServer::new(stock::StockAlertService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(StockOpLogServiceServer::new(stock::StockOpLogService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(InteractionServiceServer::new(
        interaction::InteractionService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(InteractionAdminServiceServer::new(
        interaction::InteractionAdminService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(NotificationChannelServiceServer::new(
        notification::NotificationChannelService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(NotificationRuleServiceServer::new(
        notification::NotificationRuleService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(NotificationTemplateServiceServer::new(
        notification::NotificationTemplateService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(NotificationServiceServer::new(
        notification::NotificationService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(NotificationPreferenceServiceServer::new(
        notification::NotificationPreferenceService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(OnlineSessionServiceServer::new(
        online_session::OnlineSessionService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(TenantServiceServer::new(tenant::TenantService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(OrgUnitServiceServer::new(org_unit::OrgUnitService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(PositionServiceServer::new(position::PositionService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(RoleServiceServer::new(role::RoleService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(MonitorAlertServiceServer::new(
        monitor::MonitorAlertService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(RedisCacheMonitorServiceServer::new(
        redis_cache::RedisCacheMonitorService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(ServerMonitorServiceServer::new(
        server_monitor::ServerMonitorService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(MenuServiceServer::new(menu::MenuService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(ApiServiceServer::new(api::ApiService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(PermissionServiceServer::new(perm_svc::PermissionService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(PermissionGroupServiceServer::new(
        perm_group::PermissionGroupService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(PolicyEvaluationLogServiceServer::new(
        policy_log::PolicyEvaluationLogServiceImpl {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(ApiAuditLogServiceServer::new(audit::ApiAuditLogService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(LoginAuditLogServiceServer::new(
        audit::LoginAuditLogService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(OperationAuditLogServiceServer::new(
        audit::OperationAuditLogService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(DataAccessAuditLogServiceServer::new(
        audit::DataAccessAuditLogService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(PermissionAuditLogServiceServer::new(
        audit::PermissionAuditLogService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(InternalMessageServiceServer::new(
        internal_message::InternalMessageService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(InternalMessageRecipientServiceServer::new(
        internal_message::InternalMessageRecipientService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(InternalMessageCategoryServiceServer::new(
        internal_message::InternalMessageCategoryService {
            state: std::sync::Arc::clone(&state),
        },
    ))
    .add_service(InvoiceServiceServer::new(invoice::InvoiceService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(DictTypeServiceServer::new(dict_type::DictTypeService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(DictEntryServiceServer::new(dict_entry::DictEntryService {
        state: std::sync::Arc::clone(&state),
    }))
    .add_service(LanguageServiceServer::new(language::LanguageService {
        state: std::sync::Arc::clone(&state),
    }))
}
