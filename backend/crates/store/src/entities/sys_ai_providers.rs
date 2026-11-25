//! Generated from the golden DDL (sql/schema.sql) — DO NOT EDIT.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sys_ai_providers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTimeWithTimeZone>,
    pub updated_at: Option<DateTimeWithTimeZone>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
    pub created_by: Option<i64>,
    pub updated_by: Option<i64>,
    pub deleted_by: Option<i64>,
    pub tenant_id: Option<i64>,
    pub name: Option<String>,
    pub model_type: Option<String>,
    pub model_name: Option<String>,
    pub base_url: Option<String>,
    pub organization: Option<String>,
    pub api_key: Option<String>,
    pub api_key_hint: Option<String>,
    pub local_host: Option<String>,
    pub local_port: Option<i32>,
    pub timeout_seconds: Option<i32>,
    pub system_prompt: Option<String>,
    pub is_default: Option<bool>,
    pub is_enabled: Option<bool>,
    pub remark: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
