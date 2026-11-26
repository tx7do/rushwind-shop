//! The internal-message services — the broadcast messages, the
//! per-recipient inbox rows, and the categories (the reference's
//! internal_message_service.go family). The inbox faces carry the
//! storefront read path: ListUserInbox over the recipient's rows,
//! MarkNotificationAsRead / MarkNotificationsStatus walking the
//! SENT → RECEIVED → READ state, DeleteNotificationFromInbox
//! removing the row.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{
    internal_message_categories, internal_message_recipients, internal_messages,
};
use store::paging::fetch_paged;

use shop_proto::proto::internal_message::service::v1 as imv1;
use shop_proto::proto::pagination::PagingRequest;

fn message_status_to_i32(stored: &str) -> i32 {
    use imv1::internal_message::Status;
    (match stored {
        "PUBLISHED" => Status::Published,
        "SCHEDULED" => Status::Scheduled,
        "REVOKED" => Status::Revoked,
        _ => Status::Draft,
    }) as i32
}

fn message_status_to_str(v: i32) -> Option<&'static str> {
    use imv1::internal_message::Status;
    Some(match Status::try_from(v).ok()? {
        Status::Draft => "DRAFT",
        Status::Published => "PUBLISHED",
        Status::Scheduled => "SCHEDULED",
        Status::Revoked => "REVOKED",
        _ => return None,
    })
}

fn recipient_status_to_i32(stored: &str) -> i32 {
    use imv1::internal_message_recipient::Status;
    (match stored {
        "RECEIVED" => Status::Received,
        "READ" => Status::Read,
        "REVOKED" => Status::Revoked,
        _ => Status::Sent,
    }) as i32
}

fn recipient_status_to_str(v: i32) -> Option<&'static str> {
    use imv1::internal_message_recipient::Status;
    Some(match Status::try_from(v).ok()? {
        Status::Sent => "SENT",
        Status::Received => "RECEIVED",
        Status::Read => "READ",
        Status::Revoked => "REVOKED",
        _ => return None,
    })
}

fn message_proto(r: internal_messages::Model) -> imv1::InternalMessage {
    imv1::InternalMessage {
        id: Some(r.id),
        title: Some(r.title.clone()),
        content: Some(r.content.clone()),
        status: Some(message_status_to_i32(&r.status)),
        sender_id: r.sender_id,
        sender_name: r.sender_name.clone(),
        category_id: r.category_id,
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

fn recipient_proto(r: internal_message_recipients::Model) -> imv1::InternalMessageRecipient {
    imv1::InternalMessageRecipient {
        id: Some(r.id),
        message_id: Some(r.message_id),
        recipient_user_id: Some(r.recipient_user_id),
        status: Some(recipient_status_to_i32(&r.status)),
        received_at: r.received_at.and_then(ts_to_proto),
        read_at: r.read_at.and_then(ts_to_proto),
        title: r.title.clone(),
        content: r.content.clone(),
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

fn category_proto(r: internal_message_categories::Model) -> imv1::InternalMessageCategory {
    imv1::InternalMessageCategory {
        id: Some(r.id),
        name: Some(r.name.clone()),
        code: Some(r.code.clone()),
        icon_url: r.icon_url.clone(),
        sort_order: r.sort_order.map(|v| v as u32),
        is_enabled: Some(r.is_enabled),
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

// ── InternalMessageService (the broadcast authoring face) ───────────

/// 分类名回填（参照 enrichRelations 的 CategoryName 面；best-effort）。
async fn bind_category_names(state: &AppState, items: &mut [imv1::InternalMessage]) {
    let ids: std::collections::BTreeSet<i64> = items.iter().filter_map(|m| m.category_id).collect();
    if ids.is_empty() {
        return;
    }
    let categories: std::collections::HashMap<i64, String> =
        internal_message_categories::Entity::find()
            .filter(internal_message_categories::Column::Id.is_in(ids))
            .all(&state.db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|c| (c.id, c.name))
            .collect();
    for m in items {
        if let Some(cid) = m.category_id {
            m.category_name = categories.get(&cid).cloned();
        }
    }
}

pub struct InternalMessageService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl imv1::internal_message_service_server::InternalMessageService for InternalMessageService {
    async fn list_message(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<imv1::ListInternalMessageResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            internal_messages::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        let mut items: Vec<imv1::InternalMessage> = rows.into_iter().map(message_proto).collect();
        bind_category_names(&self.state, &mut items).await;
        Ok(Response::new(imv1::ListInternalMessageResponse {
            items,
            total: total as i64,
        }))
    }

    async fn get_message(
        &self,
        request: Request<imv1::GetInternalMessageRequest>,
    ) -> Result<Response<imv1::InternalMessage>, Status> {
        let req = request.into_inner();
        let Some(imv1::get_internal_message_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = internal_messages::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("message not found"))?;
        let mut message = message_proto(row);
        bind_category_names(&self.state, std::slice::from_mut(&mut message)).await;
        Ok(Response::new(message))
    }

    async fn create_message(
        &self,
        request: Request<imv1::CreateInternalMessageRequest>,
    ) -> Result<Response<imv1::InternalMessage>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let title = data
            .title
            .clone()
            .filter(|t| !t.is_empty())
            .ok_or_else(|| bad("data.title required"))?;
        let active = internal_messages::ActiveModel {
            title: Set(title),
            content: Set(data.content.unwrap_or_default()),
            status: Set(data
                .status
                .and_then(message_status_to_str)
                .unwrap_or("DRAFT")
                .to_string()),
            sender_id: Set(data.sender_id),
            sender_name: Set(data.sender_name.clone()),
            category_id: Set(data.category_id),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        let row = active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(message_proto(row)))
    }

    async fn update_message(
        &self,
        request: Request<imv1::UpdateInternalMessageRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = internal_messages::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("message not found"))?;
        let mut active: internal_messages::ActiveModel = existing.into();
        if let Some(v) = data.title.clone() {
            active.title = Set(v);
        }
        if let Some(v) = data.content.clone() {
            active.content = Set(v);
        }
        if let Some(v) = data.status.and_then(message_status_to_str) {
            active.status = Set(v.to_string());
        }
        active.updated_at = Set(Some(store::now()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete_message(
        &self,
        request: Request<imv1::DeleteInternalMessageRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(imv1::delete_internal_message_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        internal_messages::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

// ── InternalMessageRecipientService (the inbox face) ────────────────

/// The operator off the forwarded x-user-id metadata.
fn operator_of<T>(request: &Request<T>) -> i64 {
    request
        .metadata()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0)
}

pub struct InternalMessageRecipientService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl imv1::internal_message_recipient_service_server::InternalMessageRecipientService
    for InternalMessageRecipientService
{
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<imv1::ListInternalMessageRecipientResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            internal_message_recipients::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(imv1::ListInternalMessageRecipientResponse {
            items: rows.into_iter().map(recipient_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<imv1::CountInternalMessageRecipientResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            internal_message_recipients::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(imv1::CountInternalMessageRecipientResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<imv1::GetInternalMessageRecipientRequest>,
    ) -> Result<Response<imv1::InternalMessageRecipient>, Status> {
        let req = request.into_inner();
        let Some(imv1::get_internal_message_recipient_request::QueryBy::Id(id)) = req.query_by
        else {
            return Err(bad("query_by required"));
        };
        let row = internal_message_recipients::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("recipient row not found"))?;
        Ok(Response::new(recipient_proto(row)))
    }

    async fn create(
        &self,
        request: Request<imv1::CreateInternalMessageRecipientRequest>,
    ) -> Result<Response<imv1::InternalMessageRecipient>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let message_id = data
            .message_id
            .ok_or_else(|| bad("data.messageId required"))?;
        let recipient_user_id = data
            .recipient_user_id
            .ok_or_else(|| bad("data.recipientUserId required"))?;
        let active = internal_message_recipients::ActiveModel {
            message_id: Set(message_id),
            recipient_user_id: Set(recipient_user_id),
            status: Set(data
                .status
                .and_then(recipient_status_to_str)
                .unwrap_or("SENT")
                .to_string()),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        let row = active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(recipient_proto(row)))
    }

    async fn update(
        &self,
        request: Request<imv1::UpdateInternalMessageRecipientRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = internal_message_recipients::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("recipient row not found"))?;
        let mut active: internal_message_recipients::ActiveModel = existing.into();
        if let Some(v) = data.status.and_then(recipient_status_to_str) {
            active.status = Set(v.to_string());
        }
        active.updated_at = Set(Some(store::now()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<imv1::DeleteInternalMessageRecipientRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(imv1::delete_internal_message_recipient_request::QueryBy::Id(id)) = req.query_by
        else {
            return Err(bad("query_by required"));
        };
        internal_message_recipients::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn get_internal_message_recipients_by_ids(
        &self,
        request: Request<imv1::GetInternalMessageRecipientsByIdsRequest>,
    ) -> Result<Response<imv1::ListInternalMessageRecipientResponse>, Status> {
        let req = request.into_inner();
        let rows = internal_message_recipients::Entity::find()
            .filter(internal_message_recipients::Column::Id.is_in(req.ids))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let total = rows.len() as i64;
        Ok(Response::new(imv1::ListInternalMessageRecipientResponse {
            items: rows.into_iter().map(recipient_proto).collect(),
            total,
        }))
    }

    /// The storefront inbox: the caller's rows (the operator rides the
    /// forwarded x-user-id metadata).
    async fn list_user_inbox(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<imv1::ListUserInboxResponse>, Status> {
        let op = operator_of(&request);
        if op == 0 {
            return Err(bad("operator identity required (x-user-id)"));
        }
        let (rows, total) = fetch_paged(
            &self.state.db,
            internal_message_recipients::Entity::find()
                .filter(internal_message_recipients::Column::RecipientUserId.eq(op)),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(imv1::ListUserInboxResponse {
            items: rows.into_iter().map(recipient_proto).collect(),
            total: total as i64,
        }))
    }

    async fn delete_notification_from_inbox(
        &self,
        request: Request<imv1::DeleteNotificationFromInboxRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let op = operator_of(&request);
        let req = request.into_inner();
        if op == 0 {
            return Err(bad("operator identity required (x-user-id)"));
        }
        let ids = req.recipient_ids;
        internal_message_recipients::Entity::delete_many()
            .filter(internal_message_recipients::Column::RecipientUserId.eq(op))
            .filter(internal_message_recipients::Column::Id.is_in(ids))
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn mark_notification_as_read(
        &self,
        request: Request<imv1::MarkNotificationAsReadRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let op = operator_of(&request);
        let req = request.into_inner();
        // The contract carries the user explicitly; the forwarded
        // operator is the fallback (the storefront proxy stamps it).
        let user = if req.user_id != 0 { req.user_id } else { op };
        if user == 0 {
            return Err(bad("user_id required (or forward x-user-id)"));
        }
        let _ = user;
        internal_message_recipients::Entity::update_many()
            .col_expr(
                internal_message_recipients::Column::Status,
                sea_orm::sea_query::Expr::value("READ"),
            )
            .col_expr(
                internal_message_recipients::Column::ReadAt,
                sea_orm::sea_query::Expr::value(store::now()),
            )
            .filter(internal_message_recipients::Column::RecipientUserId.eq(req.user_id))
            .filter(
                internal_message_recipients::Column::Id
                    .is_in(req.recipient_ids.into_iter().collect::<Vec<i64>>()),
            )
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn mark_notifications_status(
        &self,
        request: Request<imv1::MarkNotificationsStatusRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let op = operator_of(&request);
        let req = request.into_inner();
        let user = if req.user_id != 0 { req.user_id } else { op };
        if user == 0 {
            return Err(bad("user_id required (or forward x-user-id)"));
        }
        let _ = user;
        let new_status = recipient_status_to_str(req.new_status)
            .unwrap_or("RECEIVED")
            .to_string();
        internal_message_recipients::Entity::update_many()
            .col_expr(
                internal_message_recipients::Column::Status,
                sea_orm::sea_query::Expr::value(new_status),
            )
            .filter(internal_message_recipients::Column::RecipientUserId.eq(req.user_id))
            .filter(
                internal_message_recipients::Column::Id
                    .is_in(req.recipient_ids.into_iter().collect::<Vec<i64>>()),
            )
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

// ── InternalMessageCategoryService ──────────────────────────────────

pub struct InternalMessageCategoryService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl imv1::internal_message_category_service_server::InternalMessageCategoryService
    for InternalMessageCategoryService
{
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<imv1::ListInternalMessageCategoryResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            internal_message_categories::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(imv1::ListInternalMessageCategoryResponse {
            items: rows.into_iter().map(category_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<imv1::CountInternalMessageCategoryResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            internal_message_categories::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(imv1::CountInternalMessageCategoryResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<imv1::GetInternalMessageCategoryRequest>,
    ) -> Result<Response<imv1::InternalMessageCategory>, Status> {
        let req = request.into_inner();
        let Some(imv1::get_internal_message_category_request::QueryBy::Id(id)) = req.query_by
        else {
            return Err(bad("query_by required"));
        };
        let row = internal_message_categories::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("category not found"))?;
        Ok(Response::new(category_proto(row)))
    }

    async fn create(
        &self,
        request: Request<imv1::CreateInternalMessageCategoryRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let name = data
            .name
            .clone()
            .filter(|n| !n.is_empty())
            .ok_or_else(|| bad("data.name required"))?;
        let code = data
            .code
            .clone()
            .filter(|c| !c.is_empty())
            .ok_or_else(|| bad("data.code required"))?;
        let active = internal_message_categories::ActiveModel {
            name: Set(name),
            code: Set(code),
            icon_url: Set(data.icon_url.clone()),
            sort_order: Set(data.sort_order.map(|v| v as i32)),
            is_enabled: Set(data.is_enabled.unwrap_or(true)),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<imv1::UpdateInternalMessageCategoryRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = internal_message_categories::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("category not found"))?;
        let mut active: internal_message_categories::ActiveModel = existing.into();
        if let Some(v) = data.name.clone() {
            active.name = Set(v);
        }
        if let Some(v) = data.is_enabled {
            active.is_enabled = Set(v);
        }
        active.updated_at = Set(Some(store::now()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<imv1::DeleteInternalMessageCategoryRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(imv1::delete_internal_message_category_request::QueryBy::Id(id)) = req.query_by
        else {
            return Err(bad("query_by required"));
        };
        internal_message_categories::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}
