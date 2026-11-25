//! The server layer — the gRPC face assembly (the tonic registry),
//! mirroring the reference's `internal/server`. The service modules
//! live in [`crate::service`]; this module owns registration only.

use std::sync::Arc;

use crate::service::{self, authentication_service as authentication, brand_service as brand, cart_service as cart, comment_service as comment, category_service as category,
    coupon_service as coupon, groupon_service as groupon, identity_service as identity,
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
use shop_proto::proto::order::service::v1::{
    order_item_service_server::OrderItemServiceServer, order_service_server::OrderServiceServer,
};
use shop_proto::proto::address::service::v1::shipping_address_service_server::ShippingAddressServiceServer;
use shop_proto::proto::catalog::service::v1::groupon_activity_service_server::GrouponActivityServiceServer;
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
}
