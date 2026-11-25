//! The server layer — the gRPC face assembly (the tonic registry),
//! mirroring the reference's `internal/server`. The service modules
//! live in [`crate::service`]; this module owns registration only.

use std::sync::Arc;

use crate::service::{self, authentication_service as authentication, brand_service as brand, cart_service as cart, category_service as category};
use crate::state::AppState;
use shop_proto::proto::authentication::service::v1::authentication_service_server::AuthenticationServiceServer;
use shop_proto::proto::cart::service::v1::{
    cart_item_service_server::CartItemServiceServer, cart_service_server::CartServiceServer,
};
use shop_proto::proto::catalog::service::v1::{
    brand_service_server::BrandServiceServer, category_service_server::CategoryServiceServer,
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
        .add_service(CartItemServiceServer::new(cart::CartItemService { state }))
}
