//! Generated from the golden DDL (sql/schema.sql) — DO NOT EDIT.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "mall_coupon_templates")]
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
    pub discount_type: String,
    pub discount_value: Option<i64>,
    pub discount_percentage: Option<i32>,
    pub valid_from: Option<DateTimeWithTimeZone>,
    pub valid_until: Option<DateTimeWithTimeZone>,
    pub max_redemptions: Option<i32>,
    pub max_redemptions_per_user: Option<i32>,
    pub redeemed_count: Option<i32>,
    pub status: String,
    pub claimable: Option<bool>,
    pub points_price: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
