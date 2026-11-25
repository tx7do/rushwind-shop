//! The domain gRPC service implementations. A service absent from the
//! served list (or a method left to its generated default) answers
//! Unimplemented — the reference's servers register every module the
//! same way and land their logic module by module.

pub mod authentication_service;
pub mod brand_service;
pub mod cart_service;
pub mod coupon_service;
pub mod category_service;
pub mod groupon_service;
pub mod identity_service;
pub mod order_service;
pub mod payment_service;
pub mod product_service;
pub mod shipping_service;
pub mod wishlist_service;
pub mod sku_service;
