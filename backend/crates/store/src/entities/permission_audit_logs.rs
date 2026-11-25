//! Generated from the golden DDL (sql/schema.sql) — DO NOT EDIT.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sys_permission_audit_logs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTimeWithTimeZone>,
    pub tenant_id: Option<i64>,
    pub operator_id: Option<i64>,
    pub operator_name: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub action: Option<String>,
    #[sea_orm(column_type = "Json")]
    pub old_value: Option<Json>,
    #[sea_orm(column_type = "Json")]
    pub new_value: Option<Json>,
    pub ip_address: Option<String>,
    pub request_id: Option<String>,
    #[sea_orm(column_type = "Json")]
    pub payload: Option<Json>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
