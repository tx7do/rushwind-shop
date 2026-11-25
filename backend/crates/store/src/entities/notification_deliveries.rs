//! Generated from the golden DDL (sql/schema.sql) — DO NOT EDIT.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "mall_notification_deliveries")]
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
    pub event_type: String,
    pub channel: String,
    pub channel_id: Option<i64>,
    pub recipient_user_id: Option<i64>,
    pub target: Option<String>,
    pub status: String,
    #[sea_orm(column_type = "Text")]
    pub last_error: Option<String>,
    pub sent_at: Option<DateTimeWithTimeZone>,
    pub related_id: Option<i64>,
    pub request_id: Option<String>,
    pub attempts: Option<i32>,
    pub receipt_provider_id: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
