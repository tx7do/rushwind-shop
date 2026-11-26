//! The comment service — the product reviews (the reference's
//! comment_service.go). The rating/summary faces aggregate the
//! APPROVED rows of the product: the average star rating, and the
//! storefront review summary with its availability verdict (the
//! comment exists and carries enough content for a review snippet).

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::comments;
use store::paging::fetch_paged;

use shop_proto::proto::comment::service::v1 as commentv1;
use shop_proto::proto::pagination::PagingRequest;

fn status_to_str(v: i32) -> Option<String> {
    use commentv1::comment::Status;
    Some(match Status::try_from(v).ok()? {
        Status::Pending => "STATUS_PENDING".to_string(),
        Status::Approved => "STATUS_APPROVED".to_string(),
        Status::Rejected => "STATUS_REJECTED".to_string(),
        Status::Spam => "STATUS_SPAM".to_string(),
        _ => return None,
    })
}

#[allow(dead_code)]
fn status_to_i32(stored: &str) -> Option<i32> {
    use commentv1::comment::Status;
    Some(match stored {
        "STATUS_PENDING" => Status::Pending,
        "STATUS_APPROVED" => Status::Approved,
        "STATUS_REJECTED" => Status::Rejected,
        "STATUS_SPAM" => Status::Spam,
        _ => return None,
    } as i32)
}

fn comment_proto(r: comments::Model) -> commentv1::Comment {
    commentv1::Comment {
        id: Some(r.id),
        content_type: Some(commentv1::comment::ContentType::Product as i32),
        object_id: Some(r.object_id),
        content: r.content.clone(),
        status: status_to_i32(&r.status),
        rating: r.rating.map(|v| v as u32),
        parent_id: r.parent_id,
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

pub struct CommentService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl commentv1::comment_service_server::CommentService for CommentService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<commentv1::ListCommentResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            comments::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(commentv1::ListCommentResponse {
            items: rows.into_iter().map(comment_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<commentv1::CountCommentResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            comments::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(commentv1::CountCommentResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<commentv1::GetCommentRequest>,
    ) -> Result<Response<commentv1::Comment>, Status> {
        let req = request.into_inner();
        let Some(commentv1::get_comment_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = comments::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("comment not found"))?;
        Ok(Response::new(comment_proto(row)))
    }

    async fn create(
        &self,
        request: Request<commentv1::CreateCommentRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let object_id = data
            .object_id
            .ok_or_else(|| bad("data.objectId required"))?;
        let active = comments::ActiveModel {
            content_type: Set("CONTENT_TYPE_PRODUCT".to_string()),
            object_id: Set(object_id),
            content: Set(data.content.clone()),
            status: Set(data
                .status
                .and_then(status_to_str)
                .unwrap_or_else(|| "STATUS_APPROVED".to_string())),
            rating: Set(data.rating.map(|v| v as i32)),
            parent_id: Set(data.parent_id.filter(|v| *v != 0)),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<commentv1::UpdateCommentRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = comments::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("comment not found"))?;
        let mut active: comments::ActiveModel = existing.into();
        if let Some(v) = data.content.clone() {
            active.content = Set(Some(v));
        }
        if let Some(v) = data.status.and_then(status_to_str) {
            active.status = Set(v);
        }
        if let Some(v) = data.rating {
            active.rating = Set(Some(v as i32));
        }
        active.updated_at = Set(Some(store::now()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<commentv1::DeleteCommentRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(commentv1::delete_comment_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        comments::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The storefront rating face: the average star and the count over
    /// the product's APPROVED comments.
    async fn get_product_rating(
        &self,
        request: Request<commentv1::GetProductRatingRequest>,
    ) -> Result<Response<commentv1::ProductRatingSummary>, Status> {
        let req = request.into_inner();
        let rows = comments::Entity::find()
            .filter(comments::Column::ContentType.eq("CONTENT_TYPE_PRODUCT"))
            .filter(comments::Column::ObjectId.eq(req.product_id))
            .filter(comments::Column::Status.eq("STATUS_APPROVED"))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let count = rows.len() as i64;
        let average = if count == 0 {
            0.0
        } else {
            let sum: i64 = rows.iter().map(|r| r.rating.unwrap_or(0) as i64).sum();
            sum as f64 / count as f64
        };
        Ok(Response::new(commentv1::ProductRatingSummary {
            average,
            count,
        }))
    }

    /// The storefront review-summary face: a one-line verdict over the
    /// product's approved reviews (the count drives the availability
    /// — no reviews means the storefront renders its placeholder).
    async fn get_product_review_summary(
        &self,
        request: Request<commentv1::GetProductReviewSummaryRequest>,
    ) -> Result<Response<commentv1::ProductReviewSummary>, Status> {
        let req = request.into_inner();
        let rows = comments::Entity::find()
            .filter(comments::Column::ContentType.eq("CONTENT_TYPE_PRODUCT"))
            .filter(comments::Column::ObjectId.eq(req.product_id))
            .filter(comments::Column::Status.eq("STATUS_APPROVED"))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let count = rows.len() as u32;
        let available = count > 0;
        let summary = if available {
            let avg: f64 = {
                let sum: i64 = rows.iter().map(|r| r.rating.unwrap_or(0) as i64).sum();
                sum as f64 / rows.len() as f64
            };
            format!("{:.1} stars from {} reviews", avg, count)
        } else {
            String::new()
        };
        Ok(Response::new(commentv1::ProductReviewSummary {
            summary,
            comment_count: count,
            available,
        }))
    }
}
