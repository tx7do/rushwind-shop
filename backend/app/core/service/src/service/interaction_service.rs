//! The interaction services — the likes and their counters (the
//! reference's interaction_service.go / interaction_admin_service.go).
//! The like rows carry the per-user verdict; the counters aggregate
//! per (target, metric). Like/Unlike take the operator off the
//! forwarded x-user-id metadata; GetCounts is the anonymous face the
//! storefront whitelist carries.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, AppState};
use store::entities::{interaction_counters, interaction_likes};

use shop_proto::proto::interaction::service::v1 as interv1;

fn target_to_str(v: i32) -> &'static str {
    use interv1::TargetType;
    match interv1::TargetType::try_from(v).ok() {
        Some(TargetType::Comment) => "TARGET_TYPE_COMMENT",
        _ => "TARGET_TYPE_COMMENT",
    }
}

fn metric_to_str(v: i32) -> &'static str {
    use interv1::CounterMetric;
    match interv1::CounterMetric::try_from(v).ok() {
        Some(CounterMetric::Like) => "COUNTER_METRIC_LIKE",
        _ => "COUNTER_METRIC_LIKE",
    }
}

/// The operator off the forwarded x-user-id metadata.
fn operator_of<T>(request: &Request<T>) -> i64 {
    request
        .metadata()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0)
}

/// The counter's atomic-ish tick (the read-modify-write runs inside the
/// row's unique index; a race loses at most one tick on a hot counter).
async fn bump_counter(
    db: &sea_orm::DatabaseConnection,
    target_type: &str,
    target_id: i64,
    metric: &str,
    delta: i64,
) -> Result<i64, Status> {
    let existing = interaction_counters::Entity::find()
        .filter(interaction_counters::Column::TargetType.eq(target_type))
        .filter(interaction_counters::Column::TargetId.eq(target_id))
        .filter(interaction_counters::Column::Metric.eq(metric))
        .one(db)
        .await
        .map_err(db_status)?;
    if let Some(row) = existing {
        let mut active: interaction_counters::ActiveModel = row.into();
        let next = (active.count.clone().take().unwrap_or(0) + delta).max(0);
        active.count = Set(next);
        active.updated_at = Set(Some(store::now().into()));
        active.update(db).await.map_err(db_status)?;
        Ok(next)
    } else {
        let active = interaction_counters::ActiveModel {
            target_type: Set(target_type.to_string()),
            target_id: Set(target_id),
            metric: Set(metric.to_string()),
            count: Set(delta.max(0)),
            tenant_id: Set(Some(0)),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        active.insert(db).await.map_err(db_status)?;
        Ok(delta.max(0))
    }
}

pub struct InteractionService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl interv1::interaction_service_server::InteractionService for InteractionService {
    async fn like(
        &self,
        request: Request<interv1::LikeRequest>,
    ) -> Result<Response<interv1::LikeResponse>, Status> {
        let op = operator_of(&request);
        let req = request.into_inner();
        if op == 0 {
            return Err(bad("operator identity required (x-user-id)"));
        }
        let target_type = target_to_str(req.target_type);
        let target_id = req.target_id;
        // The idempotent like: an existing row keeps the verdict true
        // and the counter steady.
        let existing = interaction_likes::Entity::find()
            .filter(interaction_likes::Column::UserId.eq(op))
            .filter(interaction_likes::Column::TargetType.eq(target_type))
            .filter(interaction_likes::Column::TargetId.eq(target_id))
            .one(&self.state.db)
            .await
            .map_err(db_status)?;
        let count = if existing.is_some() {
            interaction_counters::Entity::find()
                .filter(interaction_counters::Column::TargetType.eq(target_type))
                .filter(interaction_counters::Column::TargetId.eq(target_id))
                .filter(interaction_counters::Column::Metric.eq("COUNTER_METRIC_LIKE"))
                .one(&self.state.db)
                .await
                .map_err(db_status)?
                .map(|c| c.count)
                .unwrap_or(0)
        } else {
            let active = interaction_likes::ActiveModel {
                user_id: Set(op),
                target_type: Set(target_type.to_string()),
                target_id: Set(target_id),
                created_at: Set(Some(store::now().into())),
                ..Default::default()
            };
            active.insert(&self.state.db).await.map_err(db_status)?;
            bump_counter(&self.state.db, target_type, target_id, "COUNTER_METRIC_LIKE", 1)
                .await?
        };
        Ok(Response::new(interv1::LikeResponse {
            liked: true,
            like_count: count as i32,
        }))
    }

    async fn unlike(
        &self,
        request: Request<interv1::LikeRequest>,
    ) -> Result<Response<interv1::LikeResponse>, Status> {
        let op = operator_of(&request);
        let req = request.into_inner();
        if op == 0 {
            return Err(bad("operator identity required (x-user-id)"));
        }
        let target_type = target_to_str(req.target_type);
        let target_id = req.target_id;
        let row = interaction_likes::Entity::find()
            .filter(interaction_likes::Column::UserId.eq(op))
            .filter(interaction_likes::Column::TargetType.eq(target_type))
            .filter(interaction_likes::Column::TargetId.eq(target_id))
            .one(&self.state.db)
            .await
            .map_err(db_status)?;
        let count = if let Some(row) = row {
            let model: interaction_likes::ActiveModel = row.into();
            model.delete(&self.state.db).await.map_err(db_status)?;
            bump_counter(&self.state.db, target_type, target_id, "COUNTER_METRIC_LIKE", -1)
                .await?
        } else {
            // The idempotent unlike of a never-liked target.
            interaction_counters::Entity::find()
                .filter(interaction_counters::Column::TargetType.eq(target_type))
                .filter(interaction_counters::Column::TargetId.eq(target_id))
                .filter(interaction_counters::Column::Metric.eq("COUNTER_METRIC_LIKE"))
                .one(&self.state.db)
                .await
                .map_err(db_status)?
                .map(|c| c.count)
                .unwrap_or(0)
        };
        Ok(Response::new(interv1::LikeResponse {
            liked: false,
            like_count: count as i32,
        }))
    }

    /// The per-user verdict batch: which of the targets the caller has
    /// liked.
    async fn get_interaction_status(
        &self,
        request: Request<interv1::GetInteractionStatusRequest>,
    ) -> Result<Response<interv1::GetInteractionStatusResponse>, Status> {
        let op = operator_of(&request);
        let req = request.into_inner();
        if op == 0 {
            return Err(bad("operator identity required (x-user-id)"));
        }
        let target_type = target_to_str(req.target_type);
        let rows = interaction_likes::Entity::find()
            .filter(interaction_likes::Column::UserId.eq(op))
            .filter(interaction_likes::Column::TargetType.eq(target_type))
            .filter(interaction_likes::Column::TargetId.is_in(req.target_ids.clone()))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let liked: std::collections::HashSet<i64> =
            rows.into_iter().map(|r| r.target_id).collect();
        let statuses = req
            .target_ids
            .into_iter()
            .map(|id| {
                (
                    id,
                    interv1::InteractionStatus {
                        liked: liked.contains(&id),
                    },
                )
            })
            .collect();
        Ok(Response::new(interv1::GetInteractionStatusResponse {
            statuses,
        }))
    }

    /// The anonymous count face (the storefront whitelist carries it).
    async fn get_counts(
        &self,
        request: Request<interv1::GetCountsRequest>,
    ) -> Result<Response<interv1::GetCountsResponse>, Status> {
        let req = request.into_inner();
        let target_type = target_to_str(req.target_type);
        // The requested metrics ride as a repeated enum; the like
        // metric is the one this deployment carries counters for.
        let metrics: Vec<String> = if req.metrics.is_empty() {
            vec!["COUNTER_METRIC_LIKE".to_string()]
        } else {
            req.metrics
                .iter()
                .map(|m| metric_to_str(*m).to_string())
                .collect()
        };
        let rows = interaction_counters::Entity::find()
            .filter(interaction_counters::Column::TargetType.eq(target_type))
            .filter(interaction_counters::Column::TargetId.is_in(req.target_ids.clone()))
            .filter(interaction_counters::Column::Metric.is_in(metrics.clone()))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let mut by_key: std::collections::HashMap<(i64, String), i64> =
            std::collections::HashMap::new();
        for r in rows {
            by_key.insert((r.target_id, r.metric), r.count);
        }
        let counts = req
            .target_ids
            .into_iter()
            .map(|id| {
                let per_metric = metrics
                    .iter()
                    .map(|m| {
                        use interv1::CounterMetric;
                        let metric_i32 = if m == "COUNTER_METRIC_LIKE" {
                            CounterMetric::Like as i32
                        } else {
                            CounterMetric::Unspecified as i32
                        };
                        interv1::MetricCount {
                            metric: metric_i32,
                            count: by_key.get(&(id, m.clone())).copied().unwrap_or(0),
                        }
                    })
                    .collect();
                (id, interv1::CountMap { counts: per_metric })
            })
            .collect();
        Ok(Response::new(interv1::GetCountsResponse { counts }))
    }
}

pub struct InteractionAdminService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl interv1::interaction_admin_service_server::InteractionAdminService
    for InteractionAdminService
{
    async fn purge_target_interactions(
        &self,
        request: Request<interv1::PurgeTargetInteractionsRequest>,
    ) -> Result<Response<interv1::PurgeTargetInteractionsResponse>, Status> {
        let req = request.into_inner();
        let target_type = target_to_str(req.target_type);
        let target_id = req.target_id;
        interaction_likes::Entity::delete_many()
            .filter(interaction_likes::Column::TargetType.eq(target_type))
            .filter(interaction_likes::Column::TargetId.eq(target_id))
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        // The purge clears the like rows; the counter recount stays
        // for the audit trail (the admin's ResetCounter zeroes it).
        interaction_counters::Entity::delete_many()
            .filter(interaction_counters::Column::TargetType.eq(target_type))
            .filter(interaction_counters::Column::TargetId.eq(target_id))
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(interv1::PurgeTargetInteractionsResponse {
            affected_rows: 0,
        }))
    }

    async fn purge_user_interactions(
        &self,
        request: Request<interv1::PurgeUserInteractionsRequest>,
    ) -> Result<Response<interv1::PurgeUserInteractionsResponse>, Status> {
        let req = request.into_inner();
        let res = interaction_likes::Entity::delete_many()
            .filter(interaction_likes::Column::UserId.eq(req.user_id))
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(interv1::PurgeUserInteractionsResponse {
            affected_rows: res.rows_affected as u32,
        }))
    }

    async fn reset_counter(
        &self,
        request: Request<interv1::ResetCounterRequest>,
    ) -> Result<Response<interv1::ResetCounterResponse>, Status> {
        let req = request.into_inner();
        let target_type = target_to_str(req.target_type);
        let metric = metric_to_str(req.metric);
        let existing = interaction_counters::Entity::find()
            .filter(interaction_counters::Column::TargetType.eq(target_type))
            .filter(interaction_counters::Column::TargetId.eq(req.target_id))
            .filter(interaction_counters::Column::Metric.eq(metric))
            .one(&self.state.db)
            .await
            .map_err(db_status)?;
        let recount = match existing {
            Some(row) => {
                let mut active: interaction_counters::ActiveModel = row.into();
                active.count = Set(0);
                active.updated_at = Set(Some(store::now().into()));
                active.update(&self.state.db).await.map_err(db_status)?;
                0
            }
            None => 0,
        };
        Ok(Response::new(interv1::ResetCounterResponse { recount }))
    }
}
