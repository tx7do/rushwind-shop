//! Generated from the golden DDL (sql/schema.sql) — DO NOT EDIT.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sys_scripts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTimeWithTimeZone>,
    pub updated_at: Option<DateTimeWithTimeZone>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
    pub created_by: Option<i64>,
    pub updated_by: Option<i64>,
    pub deleted_by: Option<i64>,
    pub name: Option<String>,
    pub language: Option<String>,
    pub hook_point: Option<String>,
    pub source: Option<String>,
    pub priority: Option<i32>,
    pub description: Option<String>,
    pub critical: Option<bool>,
    pub version: Option<i32>,
    pub is_enabled: Option<bool>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
