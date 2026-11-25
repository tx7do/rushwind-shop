//! The server layer — the gRPC face assembly (the tonic registry),
//! mirroring the reference's `internal/server`. The service modules
//! live in [`crate::service`]; this module owns registration only.

use std::sync::Arc;

use crate::service::{self, authentication_service as authentication, audit_service as audit, brand_service as brand, cart_service as cart, comment_service as comment, category_service as category,
    coupon_service as coupon, groupon_service as groupon, identity_service as identity, interaction_service as interaction,
    internal_message_service as internal_message, notification_service as notification, online_session_service as online_session,
    order_service as order, payment_service as payment, product_service as product,
    shipping_service as shipping, sku_service as sku, wishlist_service as wishlist};
use crate::state::AppState;
use shop_proto::proto::authentication::service::v1::authentication_service_server::AuthenticationServiceServer;
use shop_proto::proto::wishlist::service::v1::wishlist_service_server::WishlistServiceServer;
use shop_proto::proto::shipping::service::v1::{
    shipment_service_server::ShipmentServiceServer,
    shipping_rate_service_server::ShippingRateServiceServer,
};
use shop_proto::proto::payment::service::v1::{
    payment_channel_config_service_server::PaymentChannelConfigServiceServer,
    payment_refund_service_server::PaymentRefundServiceServer,
    payment_transaction_service_server::PaymentTransactionServiceServer,
};
use shop_proto::proto::internal_message::service::v1::{
    internal_message_category_service_server::InternalMessageCategoryServiceServer,
    internal_message_recipient_service_server::InternalMessageRecipientServiceServer,
    internal_message_service_server::InternalMessageServiceServer,
};
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
use shop_proto::proto::audit::service::v1::{
    api_audit_log_service_server::ApiAuditLogServiceServer,
    data_access_audit_log_service_server::DataAccessAuditLogServiceServer,
    login_audit_log_service_server::LoginAuditLogServiceServer,
    operation_audit_log_service_server::OperationAuditLogServiceServer,
    permission_audit_log_service_server::PermissionAuditLogServiceServer,
};
use shop_proto::proto::address::service::v1::shipping_address_service_server::ShippingAddressServiceServer;
use shop_proto::proto::catalog::service::v1::groupon_activity_service_server::GrouponActivityServiceServer;
use shop_proto::proto::interaction::service::v1::{
    interaction_admin_service_server::InteractionAdminServiceServer,
    interaction_service_server::InteractionServiceServer,
};
use shop_proto::proto::identity::service::v1::{
    user_profile_service_server::UserProfileServiceServer, user_service_server::UserServiceServer,
};
use shop_proto::proto::coupon::service::v1::{
    coupon_template_service_server::CouponTemplateServiceServer,
    user_coupon_service_server::UserCouponServiceServer,
};
use shop_proto::proto::comment::service::v1::comment_service_server::CommentServiceServer;
use shop_proto::proto::cart::service::v1::{
    cart_item_service_server::CartItemServiceServer, cart_service_server::CartServiceServer,
};
use shop_proto::proto::catalog::service::v1::{
    brand_service_server::BrandServiceServer, category_service_server::CategoryServiceServer,
    product_service_server::ProductServiceServer, sku_attribute_combination_service_server::SkuAttributeCombinationServiceServer,
    sku_price_service_server::SkuPriceServiceServer, sku_service_server::SkuServiceServer,
};

/// The served domain services — one `add_service` per landed module;
/// everything still absent answers Unimplemented by construction (the
/// generated stubs), matching the reference's module-by-module landing.
pub fn registry(state: Arc<AppState>) -> tonic::service::Routes {
    tonic::service::Routes::new(AuthenticationServiceServer::new(
        authentication::AuthenticationService {
            state: std::sync::Arc::clone(&state),
        },
    ))
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
        .add_service(CouponTemplateServiceServer::new(coupon::CouponTemplateService {
            state: std::sync::Arc::clone(&state),
        }))
        .add_service(UserCouponServiceServer::new(coupon::UserCouponService {
            state: std::sync::Arc::clone(&state),
        }))
        .add_service(UserServiceServer::new(identity::UserService {
            state: std::sync::Arc::clone(&state),
        }))
        .add_service(UserProfileServiceServer::new(identity::UserProfileService {
            state: std::sync::Arc::clone(&state),
        }))
        .add_service(ShippingAddressServiceServer::new(identity::ShippingAddressService {
            state: std::sync::Arc::clone(&state),
        }))
        .add_service(GrouponActivityServiceServer::new(groupon::GrouponActivityService {
            state: std::sync::Arc::clone(&state),
        }))
        .add_service(PaymentTransactionServiceServer::new(payment::PaymentTransactionService {
            state: std::sync::Arc::clone(&state),
        }))
        .add_service(PaymentRefundServiceServer::new(payment::PaymentRefundService {
            state: std::sync::Arc::clone(&state),
        }))
        .add_service(PaymentChannelConfigServiceServer::new(
            payment::PaymentChannelConfigService {
                state: std::sync::Arc::clone(&state),
            },
        ))
        .add_service(ShippingRateServiceServer::new(shipping::ShippingRateService {
            state: std::sync::Arc::clone(&state),
        }))
        .add_service(ShipmentServiceServer::new(shipping::ShipmentService {
            state: std::sync::Arc::clone(&state),
        }))
        .add_service(WishlistServiceServer::new(wishlist::WishlistService {
            state: std::sync::Arc::clone(&state),
        }))
        .add_service(CommentServiceServer::new(comment::CommentService {
            state: std::sync::Arc::clone(&state),
        }))
        .add_service(InteractionServiceServer::new(interaction::InteractionService {
            state: std::sync::Arc::clone(&state),
        }))
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
        .add_service(NotificationRuleServiceServer::new(notification::NotificationRuleService {
            state: std::sync::Arc::clone(&state),
        }))
        .add_service(NotificationTemplateServiceServer::new(
            notification::NotificationTemplateService {
                state: std::sync::Arc::clone(&state),
            },
        ))
        .add_service(NotificationServiceServer::new(notification::NotificationService {
            state: std::sync::Arc::clone(&state),
        }))
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
        .add_service(ApiAuditLogServiceServer::new(audit::ApiAuditLogService {
            state: std::sync::Arc::clone(&state),
        }))
        .add_service(LoginAuditLogServiceServer::new(audit::LoginAuditLogService {
            state: std::sync::Arc::clone(&state),
        }))
        .add_service(OperationAuditLogServiceServer::new(audit::OperationAuditLogService {
            state: std::sync::Arc::clone(&state),
        }))
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
}
