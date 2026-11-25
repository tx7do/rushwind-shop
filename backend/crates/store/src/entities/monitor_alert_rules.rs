//! Generated from the golden DDL (sql/schema.sql) — DO NOT EDIT.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sys_monitor_alert_rules")]
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
    pub name: String,
    pub metric: String,
    pub op: String,
    pub threshold: f64,
    pub cooldown_minutes: Option<i32>,
    pub channel: Option<String>,
    pub target: Option<String>,
    pub is_enabled: Option<bool>,
    #[sea_orm(column_type = "Text")]
    pub remark: Option<String>,
    pub last_firing: bool,
    pub last_value: f64,
    pub last_alerted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
