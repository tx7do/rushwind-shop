//! The groupon services — the activities and their buy groups (the
//! reference's groupon_service.go). The activity face is admin CRUD
//! plus the storefront's ListActiveGroupons; the group face reads the
//! OPENING/SUCCESS/FAILED rows per activity. The join/launch write
//! path rides the order flow (the leader order materializes the
//! group) and stays on its generated stub until that phase.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{groupon_activities, groupon_groups};
use store::paging::fetch_paged;

use shop_proto::proto::catalog::service::v1 as catalogv1;
use shop_proto::proto::pagination::PagingRequest;

fn activity_proto(r: groupon_activities::Model) -> catalogv1::GrouponActivity {
    catalogv1::GrouponActivity {
        id: Some(r.id),
        name: Some(r.name.clone()),
        sku_id: Some(r.sku_id),
        group_price: Some(r.group_price),
        group_size: Some(r.group_size),
        group_valid_hours: r.group_valid_hours,
        per_user_limit: r.per_user_limit,
        valid_from: r.valid_from.and_then(ts_to_proto),
        valid_until: r.valid_until.and_then(ts_to_proto),
        currency: Some(r.currency.clone()),
        status: Some(if r.status == "ACTIVE" {
            catalogv1::groupon_activity::Status::Active as i32
        } else {
            catalogv1::groupon_activity::Status::Inactive as i32
        }),
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

fn group_proto(r: groupon_groups::Model) -> catalogv1::GrouponGroup {
    catalogv1::GrouponGroup {
        id: Some(r.id),
        activity_id: Some(r.activity_id),
        leader_order_id: r.leader_order_id,
        member_count: r.member_count,
        paid_count: r.paid_count,
        target_size: Some(r.target_size),
        status: Some(match r.status.as_str() {
            "SUCCESS" => catalogv1::groupon_group::Status::Success,
            "FAILED" => catalogv1::groupon_group::Status::Failed,
            _ => catalogv1::groupon_group::Status::Opening,
        } as i32),
        expire_at: r.expire_at.and_then(ts_to_proto),
        created_at: r.created_at.and_then(ts_to_proto),
    }
}

pub struct GrouponActivityService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl catalogv1::groupon_activity_service_server::GrouponActivityService for GrouponActivityService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::ListGrouponActivityResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            groupon_activities::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::ListGrouponActivityResponse {
            items: rows.into_iter().map(activity_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::CountGrouponActivityResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            groupon_activities::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::CountGrouponActivityResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<catalogv1::GetGrouponActivityRequest>,
    ) -> Result<Response<catalogv1::GrouponActivity>, Status> {
        let req = request.into_inner();
        let id = req.id;
        let row = groupon_activities::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("groupon activity not found"))?;
        Ok(Response::new(activity_proto(row)))
    }

    async fn create(
        &self,
        request: Request<catalogv1::CreateGrouponActivityRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let name = data
            .name
            .filter(|n| !n.is_empty())
            .ok_or_else(|| bad("data.name required"))?;
        let sku_id = data.sku_id.ok_or_else(|| bad("data.skuId required"))?;
        fn ts_from(v: pbjson_types::Timestamp) -> sea_orm::prelude::DateTimeWithTimeZone {
            use chrono::TimeZone as _;
            chrono::Utc
                .timestamp_opt(v.seconds, v.nanos.max(0) as u32)
                .single()
                .unwrap_or_else(chrono::Utc::now)
                .fixed_offset()
        }
        let active = groupon_activities::ActiveModel {
            name: Set(name),
            sku_id: Set(sku_id),
            group_price: Set(data.group_price.unwrap_or(0)),
            group_size: Set(data.group_size.unwrap_or(2).max(2)),
            group_valid_hours: Set(data.group_valid_hours),
            per_user_limit: Set(data.per_user_limit),
            valid_from: Set(data.valid_from.map(ts_from)),
            valid_until: Set(data.valid_until.map(ts_from)),
            currency: Set(data.currency.unwrap_or_else(|| "CNY".to_string())),
            status: Set(
                if data.status == Some(catalogv1::groupon_activity::Status::Inactive as i32) {
                    "INACTIVE"
                } else {
                    "ACTIVE"
                }
                .to_string(),
            ),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<catalogv1::UpdateGrouponActivityRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = groupon_activities::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("groupon activity not found"))?;
        let mut active: groupon_activities::ActiveModel = existing.into();
        if let Some(v) = data.name.clone() {
            active.name = Set(v);
        }
        if let Some(v) = data.group_price {
            active.group_price = Set(v);
        }
        if let Some(v) = data.status {
            active.status = Set(if v == catalogv1::groupon_activity::Status::Active as i32 {
                "ACTIVE"
            } else {
                "INACTIVE"
            }
            .to_string());
        }
        active.updated_at = Set(Some(store::now()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<catalogv1::DeleteGrouponActivityRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let id = req.id;
        groupon_activities::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The storefront face: the ACTIVE activities inside their window.
    async fn list_active_groupons(
        &self,
        request: Request<catalogv1::ListActiveGrouponsRequest>,
    ) -> Result<Response<catalogv1::ListGrouponActivityResponse>, Status> {
        let _ = request.into_inner();
        let rows = groupon_activities::Entity::find()
            .filter(groupon_activities::Column::Status.eq("ACTIVE"))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let now_tz: chrono::DateTime<chrono::FixedOffset> = store::now();
        let items: Vec<catalogv1::GrouponActivity> = rows
            .into_iter()
            .filter(|r| {
                r.valid_from.map(|v| v <= now_tz).unwrap_or(true)
                    && r.valid_until.map(|v| v >= now_tz).unwrap_or(true)
            })
            .map(activity_proto)
            .collect();
        let total = items.len() as i64;
        Ok(Response::new(catalogv1::ListGrouponActivityResponse {
            items,
            total,
        }))
    }

    async fn list_groups(
        &self,
        request: Request<catalogv1::ListGrouponGroupsRequest>,
    ) -> Result<Response<catalogv1::ListGrouponGroupResponse>, Status> {
        let req = request.into_inner();
        let activity_id = req.activity_id;
        let mut select = groupon_groups::Entity::find()
            .filter(groupon_groups::Column::ActivityId.eq(activity_id));
        if let Some(status) = &req.status {
            select = select.filter(groupon_groups::Column::Status.eq(status.as_str()));
        }
        let rows = select.all(&self.state.db).await.map_err(db_status)?;
        let total = rows.len() as i64;
        Ok(Response::new(catalogv1::ListGrouponGroupResponse {
            items: rows.into_iter().map(group_proto).collect(),
            total,
        }))
    }

    async fn get_group(
        &self,
        request: Request<catalogv1::GetGrouponGroupRequest>,
    ) -> Result<Response<catalogv1::GrouponGroup>, Status> {
        let req = request.into_inner();
        let id = req.id;
        let row = groupon_groups::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("groupon group not found"))?;
        Ok(Response::new(group_proto(row)))
    }
}
