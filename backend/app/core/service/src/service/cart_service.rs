//! The cart services — the user's cart row and its items (the
//! reference's cart_service.go / cart_item_service.go). The storefront
//! write face: the add-to-item path stamps the operator's user id from
//! the forwarded x-user-id metadata.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::data::cart_repo as repo;
use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{cart_items, carts};
use store::paging::fetch_paged;

use shop_proto::proto::cart::service::v1 as cartv1;
use shop_proto::proto::pagination::PagingRequest;

fn cart_proto(r: carts::Model) -> cartv1::Cart {
    cartv1::Cart {
        id: Some(r.id),
        user_id: r.user_id,
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

fn cart_item_proto(r: cart_items::Model) -> cartv1::CartItem {
    cartv1::CartItem {
        id: Some(r.id),
        cart_id: Some(r.cart_id),
        sku_id: Some(r.sku_id),
        quantity: Some(r.quantity),
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

async fn items_of(db: &sea_orm::DatabaseConnection, cart_id: i64) -> Result<Vec<cart_items::Model>, Status> {
    repo::items_of_cart(db, cart_id).await
}

/// The operator id off the forwarded metadata (0 when none rode).
fn operator_of(request: &Request<impl std::any::Any>) -> i64 {
    request
        .metadata()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0)
}

pub struct CartService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl cartv1::cart_service_server::CartService for CartService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<cartv1::ListCartResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            carts::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(cartv1::ListCartResponse {
            items: rows.into_iter().map(cart_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<cartv1::CountCartResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            carts::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(cartv1::CountCartResponse { count: total as i64 }))
    }

    async fn get(
        &self,
        request: Request<cartv1::GetCartRequest>,
    ) -> Result<Response<cartv1::Cart>, Status> {
        let req = request.into_inner();
        let Some(cartv1::get_cart_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        Ok(Response::new(cart_proto(repo::cart_by_id(&self.state.db, id).await?)))
    }

    async fn create(
        &self,
        request: Request<cartv1::CreateCartRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let user_id = data.user_id.ok_or_else(|| bad("data.userId required"))?;
        // One live cart per user: an existing row is the cart, a create
        // is idempotent against it.
        if let Some(_existing) = repo::cart_of_user(&self.state.db, user_id).await? {
            return Ok(Response::new(pbjson_types::Empty {}));
        }
        let active = carts::ActiveModel {
            user_id: Set(Some(user_id)),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        repo::insert_cart(&self.state.db, active).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn batch_create(
        &self,
        request: Request<cartv1::BatchCreateCartsRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        for data in req.items {
            let Some(user_id) = data.user_id else {
                return Err(bad("data.userId required"));
            };
            if repo::cart_of_user(&self.state.db, user_id).await?.is_some() {
                continue;
            }
            let active = carts::ActiveModel {
                user_id: Set(Some(user_id)),
                tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
                created_at: Set(Some(store::now().into())),
                ..Default::default()
            };
            repo::insert_cart(&self.state.db, active).await?;
        }
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<cartv1::UpdateCartRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let existing = repo::cart_by_id(&self.state.db, req.id).await?;
        let mut active: carts::ActiveModel = existing.into();
        if let Some(v) = req.data.and_then(|d| d.user_id) {
            active.user_id = Set(Some(v));
        }
        active.updated_at = Set(Some(store::now().into()));
        repo::update_cart(&self.state.db, active).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<cartv1::DeleteCartRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(cartv1::delete_cart_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        repo::delete_cart(&self.state.db, id).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

/// The operator stamp for item writes: the forwarded x-user-id decides
/// ownership; the cart row resolves (creating the user's cart on
/// demand when the face posts an item without one).
async fn cart_for_user(
    state: &AppState,
    user_id: i64,
    tenant_id: i64,
) -> Result<carts::Model, Status> {
    if let Some(existing) = repo::cart_of_user(&state.db, user_id).await? {
        return Ok(existing);
    }
    let active = carts::ActiveModel {
        user_id: Set(Some(user_id)),
        tenant_id: Set(Some(tenant_id)),
        created_at: Set(Some(store::now().into())),
        ..Default::default()
    };
    Ok(repo::insert_cart(&state.db, active).await?)
}

pub struct CartItemService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl cartv1::cart_item_service_server::CartItemService for CartItemService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<cartv1::ListCartItemResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            cart_items::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(cartv1::ListCartItemResponse {
            items: rows.into_iter().map(cart_item_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<cartv1::CountCartItemResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            cart_items::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(cartv1::CountCartItemResponse { count: total as i64 }))
    }

    async fn get(
        &self,
        request: Request<cartv1::GetCartItemRequest>,
    ) -> Result<Response<cartv1::CartItem>, Status> {
        let req = request.into_inner();
        let Some(cartv1::get_cart_item_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        Ok(Response::new(cart_item_proto(
            repo::cart_item_by_id(&self.state.db, id).await?,
        )))
    }

    async fn create(
        &self,
        request: Request<cartv1::CreateCartItemRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let mut req = request;
        let op = operator_of(&req);
        let req = req.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let sku_id = data.sku_id.ok_or_else(|| bad("data.skuId required"))?;
        let quantity = data.quantity.unwrap_or(1).max(1);
        // The owner rides the forwarded operator metadata only (the
        // contract's CartItem carries no user field).
        if op == 0 {
            return Err(bad("operator identity required (x-user-id)"));
        }
        let user_id = op;
        let cart = cart_for_user(&self.state, user_id, data.tenant_id.unwrap_or(0)).await?;
        // Same-sku merge: a repeat add bumps the quantity row.
        let existing = cart_items::Entity::find()
            .filter(cart_items::Column::CartId.eq(cart.id))
            .filter(cart_items::Column::SkuId.eq(sku_id))
            .one(&self.state.db)
            .await
            .map_err(db_status)?;
        if let Some(row) = existing {
            let mut active: cart_items::ActiveModel = row.into();
            active.quantity = Set(active.quantity.clone().take().unwrap_or(0) + quantity);
            active.updated_at = Set(Some(store::now().into()));
            repo::update_item(&self.state.db, active).await?;
        } else {
            let active = cart_items::ActiveModel {
                cart_id: Set(cart.id),
                user_id: Set(Some(user_id)),
                sku_id: Set(sku_id),
                quantity: Set(quantity),
                tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
                created_at: Set(Some(store::now().into())),
                ..Default::default()
            };
            repo::insert_item(&self.state.db, active).await?;
        }
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn batch_create(
        &self,
        mut request: Request<cartv1::BatchCreateCartItemsRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let op = operator_of(&request);
        let req = request.into_inner();
        if op == 0 {
            return Err(bad("operator identity required (x-user-id)"));
        }
        for data in req.items {
            let Some(sku_id) = data.sku_id else {
                return Err(bad("data.skuId required"));
            };
            let user_id = op;
            let cart = cart_for_user(&self.state, user_id, data.tenant_id.unwrap_or(0)).await?;
            let active = cart_items::ActiveModel {
                cart_id: Set(cart.id),
                user_id: Set(Some(user_id)),
                sku_id: Set(sku_id),
                quantity: Set(data.quantity.unwrap_or(1).max(1)),
                tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
                created_at: Set(Some(store::now().into())),
                ..Default::default()
            };
            repo::insert_item(&self.state.db, active).await?;
        }
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<cartv1::UpdateCartItemRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = repo::cart_item_by_id(&self.state.db, req.id).await?;
        let mut active: cart_items::ActiveModel = existing.into();
        if let Some(v) = data.quantity {
            active.quantity = Set(v.max(1));
        }
        if let Some(v) = data.sku_id {
            active.sku_id = Set(v);
        }
        active.updated_at = Set(Some(store::now().into()));
        repo::update_item(&self.state.db, active).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<cartv1::DeleteCartItemRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(cartv1::delete_cart_item_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        repo::delete_item(&self.state.db, id).await?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

// Keep the items_of helper referenced (the cart view composition joins
// with the storefront cart page).
#[allow(dead_code)]
async fn _keep(db: &sea_orm::DatabaseConnection, id: i64) -> Result<Vec<cart_items::Model>, Status> {
    items_of(db, id).await
}
