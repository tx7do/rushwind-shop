//! The domain gRPC service implementations. A service absent from the
//! served list (or a method left to its generated default) answers
//! Unimplemented — the reference's servers register every module the
//! same way and land their logic module by module.

pub mod authentication_service;
pub mod brand_service;
pub mod category_service;
