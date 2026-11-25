//! The server layer — the gRPC face assembly (the tonic registry),
//! mirroring the reference's `internal/server`. The service modules
//! live in [`crate::service`]; this module owns registration only.

use std::sync::Arc;

use crate::service::{self, authentication_service as authentication, brand_service as brand, cart_service as cart, category_service as category,
    order_service as order, product_service as product, sku_service as sku};
use crate::state::AppState;
use shop_proto::proto::authentication::service::v1::authentication_service_server::AuthenticationServiceServer;
use shop_proto::proto::order::service::v1::{
    order_item_service_server::OrderItemServiceServer, order_service_server::OrderServiceServer,
};
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
        .add_service(OrderItemServiceServer::new(order::OrderItemService { state }))
}
