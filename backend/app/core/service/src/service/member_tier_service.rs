//! The member-tier service — the tier cards (the reference's
//! member_tier_service.go): the level ladder with the spend threshold
//! and the points multiplier. The GetUserTierInfo composite face (the
//! user's lifetime spend matched onto the ladder) rides the order
//! aggregation's landing.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::member_tiers;
use store::paging::fetch_paged;

use shop_proto::proto::catalog::service::v1 as catalogv1;
use shop_proto::proto::pagination::PagingRequest;

fn tier_proto(r: member_tiers::Model) -> catalogv1::MemberTier {
    catalogv1::MemberTier {
        id: Some(r.id),
        name: Some(r.name.clone()),
        level: Some(r.level as i32),
        min_spend: Some(r.min_spend),
        points_multiplier: Some(r.points_multiplier.unwrap_or(1) as i32),
        badge_icon: r.badge_icon.clone(),
        currency: Some(r.currency.clone()),
        status: Some(if r.status == "ACTIVE" {
            catalogv1::member_tier::Status::Active
        } else {
            catalogv1::member_tier::Status::Inactive
        } as i32),
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),

        ..Default::default()
    }
}

pub struct MemberTierService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl catalogv1::member_tier_service_server::MemberTierService for MemberTierService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::ListMemberTierResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            member_tiers::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::ListMemberTierResponse {
            items: rows.into_iter().map(tier_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::CountMemberTierResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            member_tiers::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::CountMemberTierResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<catalogv1::GetMemberTierRequest>,
    ) -> Result<Response<catalogv1::MemberTier>, Status> {
        let req = request.into_inner();
        let id = req.id;
        let row = member_tiers::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("member tier not found"))?;
        Ok(Response::new(tier_proto(row)))
    }

    async fn create(
        &self,
        request: Request<catalogv1::CreateMemberTierRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let name = data
            .name
            .clone()
            .filter(|n| !n.is_empty())
            .ok_or_else(|| bad("data.name required"))?;
        let active = member_tiers::ActiveModel {
            name: Set(name),
            level: Set(data.level.unwrap_or(1) as i32),
            min_spend: Set(data.min_spend.unwrap_or(0)),
            points_multiplier: Set(Some(data.points_multiplier.unwrap_or(1) as i32)),
            badge_icon: Set(data.badge_icon.clone()),
            currency: Set(data.currency.unwrap_or_else(|| "CNY".to_string())),
            status: Set(
                if data.status == Some(catalogv1::member_tier::Status::Inactive as i32) {
                    "INACTIVE"
                } else {
                    "ACTIVE"
                }
                .to_string(),
            ),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<catalogv1::UpdateMemberTierRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = member_tiers::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("member tier not found"))?;
        let mut active: member_tiers::ActiveModel = existing.into();
        if let Some(v) = data.name.clone() {
            active.name = Set(v);
        }
        if let Some(v) = data.level {
            active.level = Set(v as i32);
        }
        if let Some(v) = data.min_spend {
            active.min_spend = Set(v);
        }
        if let Some(v) = data.points_multiplier {
            active.points_multiplier = Set(Some(v as i32));
        }
        if let Some(v) = data.status {
            active.status = Set(if v == catalogv1::member_tier::Status::Active as i32 {
                "ACTIVE"
            } else {
                "INACTIVE"
            }
            .to_string());
        }
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<catalogv1::DeleteMemberTierRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let id = req.id;
        member_tiers::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn get_user_tier_info(
        &self,
        request: Request<catalogv1::GetUserTierInfoRequest>,
    ) -> Result<Response<catalogv1::GetUserTierInfoResponse>, Status> {
        let req = request.into_inner();
        let user_id = req.user_id;
        // The composite face: the user's lifetime spend matched onto the
        // ladder — rides the order aggregation's landing.
        let _ = user_id;
        Err(Status::unimplemented(
            "get_user_tier_info rides the order aggregation's landing",
        ))
    }
}
