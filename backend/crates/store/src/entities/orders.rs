//! Generated from the golden DDL (sql/schema.sql) — DO NOT EDIT.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "mall_orders")]
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
    pub currency: String,
    pub business_ref_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub user_id: i64,
    pub total_amount: i64,
    pub original_amount: Option<i64>,
    pub discount_amount: Option<i64>,
    pub shipping_fee: Option<i64>,
    pub tax_amount: Option<i64>,
    pub shipping_region: Option<String>,
    pub status: String,
    pub recipient_name: Option<String>,
    pub recipient_phone: Option<String>,
    pub shipping_address: Option<String>,
    pub groupon_activity_id: Option<i64>,
    pub groupon_group_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
