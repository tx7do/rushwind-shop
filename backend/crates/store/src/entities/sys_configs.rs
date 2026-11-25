//! Generated from the golden DDL (sql/schema.sql) — DO NOT EDIT.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sys_configs")]
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
    pub key: Option<String>,
    pub value: Option<String>,
    pub value_type: Option<String>,
    pub is_built_in: Option<bool>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
