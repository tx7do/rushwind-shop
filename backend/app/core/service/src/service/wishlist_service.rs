//! The wishlist service — the user's product favorites (the
//! reference's wishlist_service.go). The claimant's identity rides the
//! forwarded x-user-id metadata (the same stamp the cart and coupon
//! write faces read).

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::wishlists;
use store::paging::fetch_paged;

use shop_proto::proto::pagination::PagingRequest;
use shop_proto::proto::wishlist::service::v1 as wishlistv1;

fn wishlist_proto(r: wishlists::Model) -> wishlistv1::Wishlist {
    wishlistv1::Wishlist {
        id: Some(r.id),
        user_id: Some(r.user_id),
        product_id: Some(r.product_id),
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

/// The claimant off the forwarded operator metadata.
fn operator_of<T>(request: &Request<T>) -> i64 {
    request
        .metadata()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0)
}

pub struct WishlistService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl wishlistv1::wishlist_service_server::WishlistService for WishlistService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<wishlistv1::ListWishlistResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            wishlists::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(wishlistv1::ListWishlistResponse {
            items: rows.into_iter().map(wishlist_proto).collect(),
            total: total as i64,
        }))
    }

    async fn create(
        &self,
        request: Request<wishlistv1::CreateWishlistRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let op = operator_of(&request);
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let product_id = data
            .product_id
            .ok_or_else(|| bad("data.productId required"))?;
        // The owner: the data's user, else the forwarded operator.
        let user_id = data
            .user_id
            .or((op != 0).then_some(op))
            .ok_or_else(|| bad("data.userId required (or forward the operator metadata)"))?;
        let active = wishlists::ActiveModel {
            user_id: Set(user_id),
            product_id: Set(product_id),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<wishlistv1::DeleteWishlistRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(wishlistv1::delete_wishlist_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        wishlists::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}
