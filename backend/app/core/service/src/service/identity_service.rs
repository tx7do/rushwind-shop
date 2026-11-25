//! The identity services — the user administration face (the
//! reference's user_service.go, the core CRUD: the org/position
//! association methods stay on their generated Unimplemented stubs
//! until the organization tables land), the self-profile face, and
//! the shipping-address book (the reference's
//! shipping_address_service.go).

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{shipping_addresses, sys_user_roles, sys_users};
use store::paging::fetch_paged;

use shop_proto::proto::address::service::v1 as addressv1;
use shop_proto::proto::identity::service::v1 as identityv1;
use shop_proto::proto::pagination::PagingRequest;

/// The proto User.Status enum ↔ the stored spelling (the dev seed pins
/// `NORMAL`).
fn user_status_to_i32(stored: &str) -> i32 {
    use identityv1::user::Status;
    (match stored {
        "NORMAL" => Status::Normal,
        "PENDING" => Status::Pending,
        "LOCKED" => Status::Locked,
        "EXPIRED" => Status::Expired,
        _ => Status::Disabled,
    }) as i32
}

fn user_status_to_str(v: i32) -> Option<&'static str> {
    use identityv1::user::Status;
    Some(match Status::try_from(v).ok()? {
        Status::Normal => "NORMAL",
        Status::Pending => "PENDING",
        Status::Locked => "LOCKED",
        Status::Expired => "EXPIRED",
        Status::Disabled => "DISABLED",
        _ => return None,
    })
}

fn user_proto(r: sys_users::Model) -> identityv1::User {
    identityv1::User {
        id: Some(r.id),
        username: Some(r.username.clone()),
        nickname: r.nickname.clone(),
        email: r.email.clone(),
        mobile: r.mobile.clone(),
        avatar: None,
        status: Some(user_status_to_i32(r.status.as_deref().unwrap_or("NORMAL"))),
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

/// The user row plus its role ids (the `sys_user_roles` join) — the
/// profile faces carry the roles so the portal aggregates resolve.
async fn user_with_roles(
    db: &sea_orm::DatabaseConnection,
    r: sys_users::Model,
) -> identityv1::User {
    use sea_orm::QueryOrder as _;
    let mut proto = user_proto(r);
    let role_ids: Vec<i64> = sys_user_roles::Entity::find()
        .filter(sys_user_roles::Column::UserId.eq(proto.id.unwrap_or(0)))
        .order_by_asc(sys_user_roles::Column::RoleId)
        .all(db)
        .await
        .map(|rows| rows.into_iter().map(|row| row.role_id).collect())
        .unwrap_or_default();
    proto.role_ids = role_ids;
    proto
}

// ── UserService (the admin user-administration face) ────────────────

pub struct UserService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl identityv1::user_service_server::UserService for UserService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<identityv1::ListUserResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sys_users::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(identityv1::ListUserResponse {
            items: rows.into_iter().map(user_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<identityv1::CountUserResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            sys_users::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(identityv1::CountUserResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<identityv1::GetUserRequest>,
    ) -> Result<Response<identityv1::User>, Status> {
        let req = request.into_inner();
        let Some(identityv1::get_user_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = sys_users::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("user not found"))?;
        Ok(Response::new(user_with_roles(&self.state.db, row).await))
    }

    async fn create(
        &self,
        request: Request<identityv1::CreateUserRequest>,
    ) -> Result<Response<identityv1::User>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let username = data
            .username
            .filter(|u| !u.is_empty())
            .ok_or_else(|| bad("data.username required"))?;
        let hash = rushwind_password::PasswordHasher::hash(
            &rushwind_password_argon2::Argon2Hasher::new(),
            req.password.as_deref().unwrap_or(""),
        )
        .map_err(|_| Status::internal("hash failed"))?;
        let taken = sys_users::Entity::find()
            .filter(sys_users::Column::Username.eq(&username))
            .one(&self.state.db)
            .await
            .map_err(db_status)?;
        if taken.is_some() {
            return Err(Status::already_exists("username taken"));
        }
        let active = sys_users::ActiveModel {
            username: Set(username),
            nickname: Set(data.nickname.clone()),
            email: Set(data.email.clone()),
            mobile: Set(data.mobile.clone()),
            password: Set(Some(hash)),
            status: Set(Some(
                data.status
                    .and_then(user_status_to_str)
                    .unwrap_or("NORMAL")
                    .to_string(),
            )),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        let row = active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(user_proto(row)))
    }

    async fn batch_create(
        &self,
        request: Request<identityv1::BatchCreateUsersRequest>,
    ) -> Result<Response<identityv1::BatchCreateUsersResponse>, Status> {
        let req = request.into_inner();
        let mut created = Vec::with_capacity(req.items.len());
        for data in req.items {
            let Some(username) = data.username.clone().filter(|u| !u.is_empty()) else {
                return Err(bad("data.username required"));
            };

            let taken = sys_users::Entity::find()
                .filter(sys_users::Column::Username.eq(&username))
                .one(&self.state.db)
                .await
                .map_err(db_status)?;
            if taken.is_some() {
                continue;
            }
            let hash = rushwind_password::PasswordHasher::hash(
                &rushwind_password_argon2::Argon2Hasher::new(),
                "Init@123456",
            )
            .map_err(|_| Status::internal("hash failed"))?;
            let active = sys_users::ActiveModel {
                username: Set(username),
                nickname: Set(data.nickname.clone()),
                email: Set(data.email.clone()),
                mobile: Set(data.mobile.clone()),
                password: Set(Some(hash)),
                status: Set(Some(
                    data.status
                        .and_then(user_status_to_str)
                        .unwrap_or("NORMAL")
                        .to_string(),
                )),
                tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
                created_at: Set(Some(store::now().into())),
                ..Default::default()
            };
            let row = active.insert(&self.state.db).await.map_err(db_status)?;
            created.push(row.id);
        }
        Ok(Response::new(identityv1::BatchCreateUsersResponse {
            created_ids: created,
        }))
    }

    async fn update(
        &self,
        request: Request<identityv1::UpdateUserRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = sys_users::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("user not found"))?;
        let mut active: sys_users::ActiveModel = existing.into();
        if let Some(v) = data.nickname.clone() {
            active.nickname = Set(Some(v));
        }
        if let Some(v) = data.email.clone() {
            active.email = Set(Some(v));
        }
        if let Some(v) = data.mobile.clone() {
            active.mobile = Set(Some(v));
        }
        if let Some(v) = data.status.and_then(user_status_to_str) {
            active.status = Set(Some(v.to_string()));
        }
        // The password change rides its own hash pass.
        if req
            .password
            .as_deref()
            .map(|p| !p.is_empty())
            .unwrap_or(false)
        {
            let hash = rushwind_password::PasswordHasher::hash(
                &rushwind_password_argon2::Argon2Hasher::new(),
                req.password.as_deref().unwrap_or(""),
            )
            .map_err(|_| Status::internal("hash failed"))?;
            active.password = Set(Some(hash));
        }
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<identityv1::DeleteUserRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(identityv1::delete_user_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        sys_users::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn user_exists(
        &self,
        request: Request<identityv1::UserExistsRequest>,
    ) -> Result<Response<identityv1::UserExistsResponse>, Status> {
        let req = request.into_inner();
        let row = match req.query_by {
            Some(identityv1::user_exists_request::QueryBy::Id(id)) => {
                sys_users::Entity::find_by_id(id).one(&self.state.db).await
            }
            Some(identityv1::user_exists_request::QueryBy::Username(ref name)) => {
                sys_users::Entity::find()
                    .filter(sys_users::Column::Username.eq(name))
                    .one(&self.state.db)
                    .await
            }
            _ => return Err(bad("query_by required")),
        };
        let row = row.map_err(db_status)?;
        Ok(Response::new(identityv1::UserExistsResponse {
            exist: row.is_some(),
        }))
    }
}

// ── UserProfileService (the self face — the caller's own row) ───────

pub struct UserProfileService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl identityv1::user_profile_service_server::UserProfileService for UserProfileService {
    async fn get_user(
        &self,
        request: Request<pbjson_types::Empty>,
    ) -> Result<Response<identityv1::User>, Status> {
        // The self face reads the forwarded operator (the gate's claim
        // bag rides the metadata through the BFF proxy).
        let uid: i64 = request
            .metadata()
            .get("x-user-id")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| *v > 0)
            .ok_or_else(|| Status::unauthenticated("requires caller claims metadata"))?;
        let row = sys_users::Entity::find_by_id(uid)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("user not found"))?;
        Ok(Response::new(user_with_roles(&self.state.db, row).await))
    }

    async fn update_user(
        &self,
        request: Request<identityv1::UpdateUserRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = sys_users::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("user not found"))?;
        let mut active: sys_users::ActiveModel = existing.into();
        if let Some(v) = data.nickname.clone() {
            active.nickname = Set(Some(v));
        }
        if let Some(v) = data.email.clone() {
            active.email = Set(Some(v));
        }
        if let Some(v) = data.mobile.clone() {
            active.mobile = Set(Some(v));
        }
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

// ── ShippingAddressService (the address book) ───────────────────────

fn address_proto(r: shipping_addresses::Model) -> addressv1::ShippingAddress {
    addressv1::ShippingAddress {
        id: Some(r.id),
        user_id: Some(r.user_id),
        recipient_name: Some(r.recipient_name.clone()),
        recipient_phone: Some(r.recipient_phone.clone()),
        region: r.region.clone(),
        detail_address: r.detail_address.clone(),
        postal_code: r.postal_code.clone(),
        tag: r.tag.clone(),
        is_default: r.is_default,
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

/// The default flag's exclusivity: a newly-defaulted row clears the
/// user's other defaults.
async fn clear_other_defaults(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    keep_id: i64,
) -> Result<(), Status> {
    let rows = shipping_addresses::Entity::find()
        .filter(shipping_addresses::Column::UserId.eq(user_id))
        .filter(shipping_addresses::Column::IsDefault.eq(true))
        .all(db)
        .await
        .map_err(db_status)?;
    for row in rows {
        if row.id == keep_id {
            continue;
        }
        let mut active: shipping_addresses::ActiveModel = row.into();
        active.is_default = Set(Some(false));
        active.update(db).await.map_err(db_status)?;
    }
    Ok(())
}

pub struct ShippingAddressService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl addressv1::shipping_address_service_server::ShippingAddressService for ShippingAddressService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<addressv1::ListShippingAddressResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            shipping_addresses::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(addressv1::ListShippingAddressResponse {
            items: rows.into_iter().map(address_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<addressv1::CountShippingAddressResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            shipping_addresses::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(addressv1::CountShippingAddressResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<addressv1::GetShippingAddressRequest>,
    ) -> Result<Response<addressv1::ShippingAddress>, Status> {
        let req = request.into_inner();
        let Some(addressv1::get_shipping_address_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = shipping_addresses::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("shipping address not found"))?;
        Ok(Response::new(address_proto(row)))
    }

    async fn create(
        &self,
        request: Request<addressv1::CreateShippingAddressRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let user_id = data.user_id.ok_or_else(|| bad("data.userId required"))?;
        let name = data
            .recipient_name
            .filter(|n| !n.is_empty())
            .ok_or_else(|| bad("data.recipientName required"))?;
        let phone = data
            .recipient_phone
            .filter(|p| !p.is_empty())
            .ok_or_else(|| bad("data.recipientPhone required"))?;
        let active = shipping_addresses::ActiveModel {
            user_id: Set(user_id),
            recipient_name: Set(name),
            recipient_phone: Set(phone),
            region: Set(data.region.clone()),
            detail_address: Set(data.detail_address.clone()),
            postal_code: Set(data.postal_code.clone()),
            tag: Set(data.tag.clone()),
            is_default: Set(Some(data.is_default.unwrap_or(false))),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        let row = active.insert(&self.state.db).await.map_err(db_status)?;
        if data.is_default.unwrap_or(false) {
            clear_other_defaults(&self.state.db, user_id, row.id).await?;
        }
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<addressv1::UpdateShippingAddressRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = shipping_addresses::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("shipping address not found"))?;
        let user_id = existing.user_id;
        let mut active: shipping_addresses::ActiveModel = existing.into();
        if let Some(v) = data.recipient_name.clone() {
            active.recipient_name = Set(v);
        }
        if let Some(v) = data.recipient_phone.clone() {
            active.recipient_phone = Set(v);
        }
        if let Some(v) = data.region.clone() {
            active.region = Set(Some(v));
        }
        if let Some(v) = data.detail_address.clone() {
            active.detail_address = Set(Some(v));
        }
        if let Some(v) = data.postal_code.clone() {
            active.postal_code = Set(Some(v));
        }
        if let Some(v) = data.tag.clone() {
            active.tag = Set(Some(v));
        }
        if let Some(v) = data.is_default {
            active.is_default = Set(Some(v));
        }
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        if data.is_default.unwrap_or(false) {
            clear_other_defaults(&self.state.db, user_id, req.id).await?;
        }
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<addressv1::DeleteShippingAddressRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(addressv1::delete_shipping_address_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        shipping_addresses::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}
