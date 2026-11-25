//! The app REST route pack — mounts the full app BFF route surface
//! behind the service implementations (thin gRPC proxies to the core
//! domain service) (Phase 0: the generator's null placeholders; each
//! domain lands as its real implementation replaces its null), splits
//! the auth-free public subtree from the gated one (the auth-free set),
//! composes the per-route layer stack, and hands the router to the
//! lifecycle assembler — whose edge block (the CORS policy and the
//! request budget from the server document) wraps the whole thing.
//!
//! Per-route layer composition (the `wrap` closure below): the framework
//! bind layer — body and query binding — outermost on EVERY route, and
//! the auth gate (the auth crate) composed INSIDE it on gated routes
//! only. That order is the wire contract: codec/binding failures answer
//! 400 ahead of any 401.
//!
//! The audit-write layer and the authorization engine wire here when
//! the storage phase lands; until then the gate is the protected
//! subtree's only defense.

use std::sync::Arc;

use axum::routing::MethodRouter;

use crate::state::AppState;
use auth::auth_gate;
use rushwind_http_binding::bindgate::bind_run;
use rushwind_http_binding::wire::RouteWire;
use shop_proto::pool;

/// The deployment's registered codec subtypes — the packages
/// its binary imports (the compatibility spec §2.1 register).
const REGISTERED_SUBTYPES: &[&str] = &["json", "proto", "x-www-form-urlencoded"];

/// The owning BFF package — the error tables' anchor for the gate's
/// envelope.
const PACKAGE: &str = "app.service.v1";

/// The adapter wiring the core service's ValidateToken RPC into the
/// auth gate's server-side check stage (the reference's TokenChecker
/// dials core the same way).
struct CoreTokenChecker(std::sync::Arc<crate::state::AppState>);

#[async_trait::async_trait]
impl auth::AccessTokenChecker for CoreTokenChecker {
    async fn is_valid_access_token(&self, _uid: i64, _jti: &str, token: &str) -> bool {
        let mut core = self.0.core.clone();
        let Ok(resp) = core
            .validate_token(tonic::Request::new(
                shop_proto::proto::authentication::service::v1::ValidateTokenRequest {
                    token: token.to_string(),
                    // The app BFF's session store is the ct=1 side — the
                    // validation dials the same client the token was
                    // minted against.
                    client_type: shop_proto::proto::authentication::service::v1::ClientType::App
                        as i32,
                    ..Default::default()
                },
            ))
            .await
        else {
            return false;
        };
        resp.into_inner().is_valid
    }
    async fn is_blocked_access_token(&self, _jti: &str) -> bool {
        // The whitelist check subsumes the blacklist (core returns a
        // single validity verdict over both families).
        false
    }
}

/// The app route pack: the mounted router (see the module docs).
pub fn pack(
    state: Arc<AppState>,
) -> impl Fn(
    serde_json::Value,
    rushwind_bootstrap::RouteInput,
) -> Result<rushwind_bootstrap::RouteSurface, rushwind_bootstrap::BootstrapError>
       + Send
       + Sync
       + 'static {
    move |_settings, _input| {
        Ok(rushwind_bootstrap::RouteSurface::new(build_router(
            Arc::clone(&state),
        )))
    }
}

/// Builds the mounted router. `state` carries the verification engine
/// and the server-side session store.
pub fn build_router(state: Arc<AppState>) -> axum::Router {
    let descriptor_pool = pool();
    let authenticator = Arc::clone(&state.authenticator);
    let checker = Arc::new(CoreTokenChecker(Arc::clone(&state)))
        as Arc<dyn auth::AccessTokenChecker + 'static>;

    // The per-route layer composition (see the module docs): bind layer
    // outermost always, auth gate inside it on gated routes.
    let wrap = |mr: MethodRouter, wire: &RouteWire, gated: bool| -> MethodRouter {
        let mut out = mr;
        if gated {
            let auth = Arc::clone(&authenticator);
            let checker = Arc::clone(&checker);
            let gate = axum::middleware::from_fn(move |req, next| {
                let auth = Arc::clone(&auth);
                let checker = Arc::clone(&checker);
                async move { auth_gate(auth, checker, PACKAGE, req, next).await }
            });
            out = out.layer(gate);
        }
        let input_fq = wire.input_fq;
        let body_star = wire.body_star;
        let bind = axum::middleware::from_fn(move |req, next| {
            let (fq, body) = (input_fq, body_star);
            async move { bind_run(descriptor_pool, fq, body, REGISTERED_SUBTYPES, req, next).await }
        });
        out = out.layer(bind);
        out
    };

    // The full mounted surface — the generator's null placeholders in
    // phase 0; every mount call threads both routers, and the
    // generator's auth-free table classifies each of its route
    // bindings.
    let mut router_pub = axum::Router::new();
    let mut router_gate = axum::Router::new();
    /// The uniform service mount: every service follows the same
    /// router-threading, service-construction, and wrap handshake.
    macro_rules! mount_services {
    ($($mount:ident => $ctor:expr),* $(,)?) => {
        $(
            (router_pub, router_gate) = shop_proto::gen_app::mounts::$mount(
                router_pub,
                router_gate,
                $ctor,
                &wrap,
            );
        )*
    };
}

    mount_services!(
        mount_ai_chat_service => Arc::new(crate::services::proxies::AiChatProxy { state: Arc::clone(&state) }),
        mount_authentication_service => Arc::new(crate::services::proxies::AuthenticationProxy { state: Arc::clone(&state) }),
        mount_brand_service => Arc::new(crate::services::proxies::BrandProxy { state: Arc::clone(&state) }),
        mount_cart_item_service => Arc::new(crate::services::proxies::CartItemProxy { state: Arc::clone(&state) }),
        mount_cart_service => Arc::new(crate::services::proxies::CartProxy { state: Arc::clone(&state) }),
        mount_category_service => Arc::new(crate::services::proxies::CategoryProxy { state: Arc::clone(&state) }),
        mount_comment_service => Arc::new(crate::services::proxies::CommentProxy { state: Arc::clone(&state) }),
        mount_coupon_template_service => Arc::new(crate::services::proxies::CouponTemplateProxy { state: Arc::clone(&state) }),
        mount_file_transfer_service => Arc::new(shop_proto::gen_app::nulls::NullFileTransferService),
        mount_groupon_service => Arc::new(crate::services::proxies::GrouponProxy { state: Arc::clone(&state) }),
        mount_interaction_service => Arc::new(crate::services::proxies::InteractionProxy { state: Arc::clone(&state) }),
        mount_internal_message_recipient_service => Arc::new(crate::services::proxies::InternalMessageRecipientProxy { state: Arc::clone(&state) }),
        mount_invoice_service => Arc::new(crate::services::proxies::InvoiceProxy { state: Arc::clone(&state) }),
        mount_member_tier_service => Arc::new(crate::services::proxies::MemberTierProxy { state: Arc::clone(&state) }),
        mount_order_item_service => Arc::new(crate::services::proxies::OrderItemProxy { state: Arc::clone(&state) }),
        mount_order_service => Arc::new(crate::services::proxies::OrderProxy { state: Arc::clone(&state) }),
        mount_payment_channel_service => Arc::new(crate::services::proxies::PaymentChannelProxy { state: Arc::clone(&state) }),
        mount_payment_refund_service => Arc::new(crate::services::proxies::PaymentRefundProxy { state: Arc::clone(&state) }),
        mount_payment_transaction_service => Arc::new(crate::services::proxies::PaymentTransactionProxy { state: Arc::clone(&state) }),
        mount_points_service => Arc::new(crate::services::proxies::PointsProxy { state: Arc::clone(&state) }),
        mount_product_attribute_service => Arc::new(crate::services::proxies::ProductAttributeProxy { state: Arc::clone(&state) }),
        mount_product_attribute_value_service => Arc::new(crate::services::proxies::ProductAttributeValueProxy { state: Arc::clone(&state) }),
        mount_product_service => Arc::new(crate::services::proxies::ProductProxy { state: Arc::clone(&state) }),
        mount_promotion_service => Arc::new(crate::services::proxies::PromotionProxy { state: Arc::clone(&state) }),
        mount_seckill_service => Arc::new(crate::services::proxies::SeckillProxy { state: Arc::clone(&state) }),
        mount_shipment_service => Arc::new(crate::services::proxies::ShipmentProxy { state: Arc::clone(&state) }),
        mount_shipping_address_service => Arc::new(crate::services::proxies::ShippingAddressProxy { state: Arc::clone(&state) }),
        mount_sku_attribute_combination_service => Arc::new(crate::services::proxies::SkuAttributeCombinationProxy { state: Arc::clone(&state) }),
        mount_sku_price_service => Arc::new(crate::services::proxies::SkuPriceProxy { state: Arc::clone(&state) }),
        mount_sku_service => Arc::new(crate::services::proxies::SkuProxy { state: Arc::clone(&state) }),
        mount_user_coupon_service => Arc::new(crate::services::proxies::UserCouponProxy { state: Arc::clone(&state) }),
        mount_user_profile_service => Arc::new(crate::services::proxies::UserProfileProxy { state: Arc::clone(&state) }),
        mount_wishlist_service => Arc::new(crate::services::proxies::WishlistProxy { state: Arc::clone(&state) }),
    );

    router_pub.merge(router_gate)
}
