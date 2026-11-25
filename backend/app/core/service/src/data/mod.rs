//! The data layer — one repository module per domain table, mirroring
//! the reference's `internal/data`.

pub mod tenant_repo;
pub mod org_unit_repo;
pub mod position_repo;
pub mod api_repo;
pub mod menu_repo;
pub mod permission_api_repo;
pub mod permission_group_repo;
pub mod permission_menu_repo;
pub mod permission_repo;
pub mod policy_evaluation_log_repo;
pub mod role_metadata_repo;
pub mod role_permission_repo;
pub mod role_repo;
pub mod user_role_repo;
pub mod dict_type_repo;
pub mod dict_entry_repo;
pub mod language_repo;
pub mod brand_repo;
pub mod attr_repo;
pub mod cart_repo;
pub mod category_repo;
pub mod product_repo;
