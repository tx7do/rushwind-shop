//! Generated from the golden DDL (sql/schema.sql) — DO NOT EDIT.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sys_script_logs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTimeWithTimeZone>,
    pub script_id: Option<i64>,
    pub script_name: Option<String>,
    pub language: Option<String>,
    pub trigger_type: Option<String>,
    pub hook_point: Option<String>,
    pub version: Option<i32>,
    pub success: Option<bool>,
    pub duration_ms: Option<i64>,
    pub error: Option<String>,
    pub context: Option<Json>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
