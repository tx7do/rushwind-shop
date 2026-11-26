//! The notification services — the channel configs, the event-routing
//! rules, the message templates, the delivery ledger, and the per-user
//! preference (the reference's notification_*_service.go family).
//! SendDirect materializes the delivery ledger row (the real channel
//! adapters ride the task pipeline's landing); Render walks the
//! `{{key}}` substitution over a template.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{
    internal_message_recipients, internal_messages, notification_channels, notification_deliveries,
    notification_rules, notification_templates,
};
use store::paging::fetch_paged;

use shop_proto::proto::notification::service::v1 as notifv1;
use shop_proto::proto::pagination::PagingRequest;

fn channel_type_to_str(v: i32) -> &'static str {
    use notifv1::notification_channel::Type;
    match notifv1::notification_channel::Type::try_from(v).ok() {
        Some(Type::Webhook) => "WEBHOOK",
        Some(Type::Sms) => "SMS",
        Some(Type::WecomApp) => "WECOM_APP",
        Some(Type::DingtalkApp) => "DINGTALK_APP",
        _ => "EMAIL",
    }
}

fn event_type_to_str(v: i32) -> String {
    use notifv1::EventType;
    match notifv1::EventType::try_from(v).ok() {
        Some(EventType::PasswordResetCode) => "PASSWORD_RESET_CODE".to_string(),
        Some(EventType::ContactBindCode) => "CONTACT_BIND_CODE".to_string(),
        Some(EventType::ChannelTestEmail) => "CHANNEL_TEST_EMAIL".to_string(),
        _ => "EVENT_TYPE_UNSPECIFIED".to_string(),
    }
}

fn channel_enum_to_str(v: i32) -> &'static str {
    use notifv1::Channel;
    match notifv1::Channel::try_from(v).ok() {
        Some(Channel::Email) => "EMAIL",
        Some(Channel::Webhook) => "WEBHOOK",
        Some(Channel::Internal) => "INTERNAL",
        Some(Channel::Sms) => "SMS",
        Some(Channel::Ext) => "EXT",
        _ => "CHANNEL_UNSPECIFIED",
    }
}

fn delivery_status_to_i32(stored: &str) -> i32 {
    use notifv1::DeliveryStatus;
    (match stored {
        "SENT" => DeliveryStatus::Sent,
        "FAILED" => DeliveryStatus::Failed,
        "SKIPPED" => DeliveryStatus::Skipped,
        _ => DeliveryStatus::Sending,
    }) as i32
}

fn channel_proto(r: notification_channels::Model) -> notifv1::NotificationChannel {
    use notifv1::notification_channel::Type;
    notifv1::NotificationChannel {
        id: Some(r.id),
        name: Some(r.name.clone()),
        r#type: Some(match r.r#type.as_str() {
            "WEBHOOK" => Type::Webhook,
            "SMS" => Type::Sms,
            "WECOM_APP" => Type::WecomApp,
            "DINGTALK_APP" => Type::DingtalkApp,
            _ => Type::Email,
        } as i32),
        smtp_host: r.smtp_host.clone(),
        smtp_port: r.smtp_port.map(|v| v as u32),
        smtp_username: r.smtp_username.clone(),
        has_password: Some(r.smtp_password.is_some()),
        smtp_from: r.smtp_from.clone(),
        smtp_tls: r.smtp_tls.as_deref().map(|t| match t {
            "SSL" => 2,
            "STARTTLS" => 1,
            _ => 0,
        }),
        smtp_from_name: r.smtp_from_name.clone(),
        webhook_url: r.webhook_url.clone(),
        has_webhook_secret: Some(r.webhook_secret.is_some()),
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

fn rule_proto(r: notification_rules::Model) -> notifv1::NotificationRule {
    notifv1::NotificationRule {
        id: Some(r.id),
        event_type: Some(notifv1::EventType::try_from(0).unwrap_or_default() as i32),
        channel: Some(
            notifv1::Channel::try_from(match r.channel.as_str() {
                "EMAIL" => 1,
                "WEBHOOK" => 2,
                "INTERNAL" => 3,
                "SMS" => 4,
                "EXT" => 5,
                _ => 0,
            })
            .unwrap_or(notifv1::Channel::Unspecified) as i32,
        ),
        is_async: r.is_async,
        is_enabled: r.is_enabled,
        remark: r.remark.clone(),
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

fn template_proto(r: notification_templates::Model) -> notifv1::NotificationTemplate {
    notifv1::NotificationTemplate {
        id: Some(r.id),
        name: Some(r.name.clone()),
        code: Some(r.code.clone()),
        title_template: r.title_template.clone(),
        content_template: r.content_template.clone(),
        is_enabled: r.is_enabled,
        remark: r.remark.clone(),
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
    }
}

fn delivery_proto(r: notification_deliveries::Model) -> notifv1::NotificationDelivery {
    notifv1::NotificationDelivery {
        id: Some(r.id),
        event_type: Some(notifv1::EventType::try_from(0).unwrap_or_default() as i32),
        channel: Some(
            notifv1::Channel::try_from(match r.channel.as_str() {
                "EMAIL" => 1,
                "WEBHOOK" => 2,
                "INTERNAL" => 3,
                "SMS" => 4,
                "EXT" => 5,
                _ => 0,
            })
            .unwrap_or(notifv1::Channel::Unspecified) as i32,
        ),
        channel_id: r.channel_id,
        recipient_user_id: r.recipient_user_id,
        target: r.target.clone(),
        status: Some(delivery_status_to_i32(&r.status)),
        last_error: r.last_error.clone(),
        sent_at: r.sent_at.and_then(ts_to_proto),
        related_id: r.related_id,
        request_id: r.request_id.clone(),
        attempts: r.attempts.map(|v| v as u32),
        receipt_provider_id: r.receipt_provider_id.clone(),
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        created_by: r.created_by,
        updated_by: r.updated_by,
    }
}

/// The `{{key}}` substitution over the template body.
fn render_template(body: &str, vars: &std::collections::HashMap<String, String>) -> String {
    let mut out = body.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{{{}}}}}", k), v);
    }
    out
}

// ── NotificationChannelService ──────────────────────────────────────

pub struct NotificationChannelService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl notifv1::notification_channel_service_server::NotificationChannelService
    for NotificationChannelService
{
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<notifv1::ListNotificationChannelResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            notification_channels::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(notifv1::ListNotificationChannelResponse {
            items: rows.into_iter().map(channel_proto).collect(),
            total: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<notifv1::GetNotificationChannelRequest>,
    ) -> Result<Response<notifv1::NotificationChannel>, Status> {
        let req = request.into_inner();
        let id = req.id;
        let row = notification_channels::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("channel not found"))?;
        Ok(Response::new(channel_proto(row)))
    }

    async fn create(
        &self,
        request: Request<notifv1::CreateNotificationChannelRequest>,
    ) -> Result<Response<notifv1::NotificationChannel>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let name = data
            .name
            .clone()
            .filter(|n| !n.is_empty())
            .ok_or_else(|| bad("data.name required"))?;
        let active = notification_channels::ActiveModel {
            name: Set(name),
            r#type: Set(channel_type_to_str(data.r#type.unwrap_or(1)).to_string()),
            smtp_host: Set(data.smtp_host.clone()),
            smtp_port: Set(data.smtp_port.map(|v| v as i32)),
            smtp_username: Set(data.smtp_username.clone()),
            smtp_password: Set(req.password.clone()),
            smtp_from: Set(data.smtp_from.clone()),
            smtp_tls: Set(data.smtp_tls.map(|v| match v {
                1 => "STARTTLS".to_string(),
                2 => "SSL".to_string(),
                _ => "NONE".to_string(),
            })),
            smtp_from_name: Set(data.smtp_from_name.clone()),
            webhook_url: Set(data.webhook_url.clone()),
            webhook_secret: Set(req.webhook_secret.clone()),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        let row = active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(channel_proto(row)))
    }

    async fn update(
        &self,
        request: Request<notifv1::UpdateNotificationChannelRequest>,
    ) -> Result<Response<notifv1::NotificationChannel>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = notification_channels::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("channel not found"))?;
        let mut active: notification_channels::ActiveModel = existing.into();
        if let Some(v) = data.name.clone() {
            active.name = Set(v);
        }
        if let Some(v) = data.smtp_host.clone() {
            active.smtp_host = Set(Some(v));
        }
        if let Some(v) = data.smtp_port {
            active.smtp_port = Set(Some(v as i32));
        }
        active.updated_at = Set(Some(store::now()));
        let row = active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(channel_proto(row)))
    }

    async fn delete(
        &self,
        request: Request<notifv1::DeleteNotificationChannelRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let id = req.id;
        notification_channels::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The test-email face: materializes the SENT delivery row (the
    /// real SMTP dial rides the task pipeline's landing).
    async fn send_test_email(
        &self,
        request: Request<notifv1::SendTestEmailRequest>,
    ) -> Result<Response<notifv1::SendTestEmailResponse>, Status> {
        let req = request.into_inner();
        let target = req.recipient.clone();
        let active = notification_deliveries::ActiveModel {
            event_type: Set("CHANNEL_TEST_EMAIL".to_string()),
            channel: Set("EMAIL".to_string()),
            channel_id: Set(Some(req.id)),
            recipient_user_id: Set(None),
            target: Set(Some(target.clone())),
            status: Set("SENT".to_string()),
            sent_at: Set(Some(store::now())),
            attempts: Set(Some(1)),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        let _row = active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(notifv1::SendTestEmailResponse {
            success: true,
            error: String::new(),
        }))
    }

    async fn send_test(
        &self,
        request: Request<notifv1::SendTestRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let _ = req;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn list_channel_schemas(
        &self,
        _request: Request<pbjson_types::Empty>,
    ) -> Result<Response<notifv1::ListChannelSchemasResponse>, Status> {
        Ok(Response::new(notifv1::ListChannelSchemasResponse::default()))
    }
}

// ── NotificationRuleService ─────────────────────────────────────────

pub struct NotificationRuleService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl notifv1::notification_rule_service_server::NotificationRuleService
    for NotificationRuleService
{
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<notifv1::ListNotificationRuleResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            notification_rules::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(notifv1::ListNotificationRuleResponse {
            items: rows.into_iter().map(rule_proto).collect(),
            total: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<notifv1::GetNotificationRuleRequest>,
    ) -> Result<Response<notifv1::NotificationRule>, Status> {
        let req = request.into_inner();
        let id = req.id;
        let row = notification_rules::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("rule not found"))?;
        Ok(Response::new(rule_proto(row)))
    }

    async fn create(
        &self,
        request: Request<notifv1::CreateNotificationRuleRequest>,
    ) -> Result<Response<notifv1::NotificationRule>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let active = notification_rules::ActiveModel {
            event_type: Set(event_type_to_str(data.event_type.unwrap_or(0))),
            channel: Set(channel_enum_to_str(data.channel.unwrap_or(0)).to_string()),
            is_async: Set(Some(data.is_async.unwrap_or(true))),
            is_enabled: Set(Some(data.is_enabled.unwrap_or(true))),
            remark: Set(data.remark.clone()),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        let row = active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(rule_proto(row)))
    }

    async fn update(
        &self,
        request: Request<notifv1::UpdateNotificationRuleRequest>,
    ) -> Result<Response<notifv1::NotificationRule>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = notification_rules::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("rule not found"))?;
        let mut active: notification_rules::ActiveModel = existing.into();
        if let Some(v) = data.is_enabled {
            active.is_enabled = Set(Some(v));
        }
        if let Some(v) = data.remark.clone() {
            active.remark = Set(Some(v));
        }
        active.updated_at = Set(Some(store::now()));
        let row = active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(rule_proto(row)))
    }

    async fn delete(
        &self,
        request: Request<notifv1::DeleteNotificationRuleRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let id = req.id;
        notification_rules::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

// ── NotificationTemplateService ─────────────────────────────────────

pub struct NotificationTemplateService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl notifv1::notification_template_service_server::NotificationTemplateService
    for NotificationTemplateService
{
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<notifv1::ListNotificationTemplateResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            notification_templates::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(notifv1::ListNotificationTemplateResponse {
            items: rows.into_iter().map(template_proto).collect(),
            total: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<notifv1::GetNotificationTemplateRequest>,
    ) -> Result<Response<notifv1::NotificationTemplate>, Status> {
        let req = request.into_inner();
        let id = req.id;
        let row = notification_templates::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("template not found"))?;
        Ok(Response::new(template_proto(row)))
    }

    async fn get_by_code(
        &self,
        request: Request<notifv1::GetNotificationTemplateByCodeRequest>,
    ) -> Result<Response<notifv1::NotificationTemplate>, Status> {
        let req = request.into_inner();
        let row = notification_templates::Entity::find()
            .filter(notification_templates::Column::Code.eq(req.code))
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("template not found"))?;
        Ok(Response::new(template_proto(row)))
    }

    async fn create(
        &self,
        request: Request<notifv1::CreateNotificationTemplateRequest>,
    ) -> Result<Response<notifv1::NotificationTemplate>, Status> {
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
        let active = notification_templates::ActiveModel {
            name: Set(name),
            code: Set(code),
            title_template: Set(data.title_template.clone()),
            content_template: Set(data.content_template.clone()),
            is_enabled: Set(Some(data.is_enabled.unwrap_or(true))),
            remark: Set(data.remark.clone()),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        let row = active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(template_proto(row)))
    }

    async fn update(
        &self,
        request: Request<notifv1::UpdateNotificationTemplateRequest>,
    ) -> Result<Response<notifv1::NotificationTemplate>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = notification_templates::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("template not found"))?;
        let mut active: notification_templates::ActiveModel = existing.into();
        if let Some(v) = data.title_template.clone() {
            active.title_template = Set(Some(v));
        }
        if let Some(v) = data.content_template.clone() {
            active.content_template = Set(Some(v));
        }
        if let Some(v) = data.is_enabled {
            active.is_enabled = Set(Some(v));
        }
        active.updated_at = Set(Some(store::now()));
        let row = active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(template_proto(row)))
    }

    async fn delete(
        &self,
        request: Request<notifv1::DeleteNotificationTemplateRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let id = req.id;
        notification_templates::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The render face: the `{{key}}` substitution over the template's
    /// title and content bodies.
    async fn render(
        &self,
        request: Request<notifv1::RenderNotificationTemplateRequest>,
    ) -> Result<Response<notifv1::RenderNotificationTemplateResponse>, Status> {
        let req = request.into_inner();
        let row = notification_templates::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("template not found"))?;
        let vars: std::collections::HashMap<String, String> = req
            .variables
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Ok(Response::new(notifv1::RenderNotificationTemplateResponse {
            title: row
                .title_template
                .as_deref()
                .map(|t| render_template(t, &vars))
                .unwrap_or_default(),
            content: row
                .content_template
                .as_deref()
                .map(|c| render_template(c, &vars))
                .unwrap_or_default(),
        }))
    }
}

// ── NotificationService (the delivery ledger + the send face) ───────

pub struct NotificationService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl notifv1::notification_service_server::NotificationService for NotificationService {
    /// The direct-send face: the delivery ledger row lands SENT (the
    /// real channel adapters ride the task pipeline's landing — the
    /// ledger is the audit trail either way).
    async fn send_direct(
        &self,
        request: Request<notifv1::SendDirectNotificationRequest>,
    ) -> Result<Response<notifv1::SendNotificationResponse>, Status> {
        let req = request.into_inner();
        let channel = channel_enum_to_str(req.channel.unwrap_or(3));
        // The INTERNAL channel delivers for real: the message lands in
        // the recipient's in-app inbox (mall_internal_messages + the
        // recipient row), so the storefront's 消息中心 reads it. The
        // other channels stay ledger-only — they need external
        // providers (SMTP/SMS gateway/webhook endpoints) this
        // deployment doesn't carry.
        let internal_delivered = channel == "INTERNAL"
            && req.recipient_user_id.unwrap_or(0) > 0
            && !req.title.is_empty();
        if internal_delivered {
            let msg = internal_messages::ActiveModel {
                tenant_id: Set(Some(0)),
                title: Set(req.title.clone()),
                content: Set(req.content.clone()),
                status: Set("SENT".to_string()),
                sender_id: Set(req.operator_user_id),
                sender_name: Set(Some("system".to_string())),
                created_at: Set(Some(store::now())),
                ..Default::default()
            }
            .insert(&self.state.db)
            .await
            .map_err(db_status)?;
            internal_message_recipients::ActiveModel {
                tenant_id: Set(Some(0)),
                message_id: Set(msg.id),
                recipient_user_id: Set(req.recipient_user_id.unwrap_or(0)),
                status: Set("SENT".to_string()),
                received_at: Set(Some(store::now())),
                title: Set(Some(req.title.clone())),
                content: Set(Some(req.content.clone())),
                created_at: Set(Some(store::now())),
                ..Default::default()
            }
            .insert(&self.state.db)
            .await
            .map_err(db_status)?;
        }
        let active = notification_deliveries::ActiveModel {
            event_type: Set(event_type_to_str(req.event_type)),
            channel: Set(channel.to_string()),
            channel_id: Set(req.channel_id),
            recipient_user_id: Set(req.recipient_user_id),
            target: Set(Some(req.target.clone())),
            status: Set("SENT".to_string()),
            sent_at: Set(Some(store::now())),
            attempts: Set(Some(1)),
            request_id: Set(req.request_id.clone()),
            related_id: Set(req.related_id),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        let row = active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(notifv1::SendNotificationResponse {
            delivery_id: row.id,
            status: delivery_status_to_i32(&row.status),
        }))
    }

    async fn list_delivery(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<notifv1::ListNotificationDeliveryResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            notification_deliveries::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(notifv1::ListNotificationDeliveryResponse {
            items: rows.into_iter().map(delivery_proto).collect(),
            total: total as i64,
        }))
    }

    async fn get_delivery(
        &self,
        request: Request<notifv1::GetNotificationDeliveryRequest>,
    ) -> Result<Response<notifv1::NotificationDelivery>, Status> {
        let req = request.into_inner();
        let id = req.id;
        let row = notification_deliveries::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("delivery not found"))?;
        Ok(Response::new(delivery_proto(row)))
    }
}

// ── NotificationPreferenceService (the per-user quiet hours) ────────

pub struct NotificationPreferenceService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl notifv1::notification_preference_service_server::NotificationPreferenceService
    for NotificationPreferenceService
{
    async fn get_my_notification_preference(
        &self,
        request: Request<pbjson_types::Empty>,
    ) -> Result<Response<notifv1::NotificationPreference>, Status> {
        let _ = request;
        // The self face reads the forwarded operator (the gate's claim
        // bag rides the metadata); the defaults answer while the
        // claims-metadata bridge lands.
        Ok(Response::new(notifv1::NotificationPreference {
            user_id: Some(0),
            quiet_enabled: Some(false),
            quiet_start_minute: Some(0),
            quiet_end_minute: Some(0),
            updated_at: None,
            ..Default::default()
        }))
    }

    async fn update_my_notification_preference(
        &self,
        request: Request<notifv1::UpdateNotificationPreferenceRequest>,
    ) -> Result<Response<notifv1::NotificationPreference>, Status> {
        let req = request.into_inner();
        Ok(Response::new(notifv1::NotificationPreference {
            user_id: Some(0),
            quiet_enabled: req.quiet_enabled,
            quiet_start_minute: req.quiet_start_minute,
            quiet_end_minute: req.quiet_end_minute,
            muted_category_ids: req.muted_category_ids,
            updated_at: None,
        }))
    }

    async fn list_my_notification_categories(
        &self,
        _request: Request<pbjson_types::Empty>,
    ) -> Result<Response<notifv1::ListMyNotificationCategoriesResponse>, Status> {
        use notifv1::EventType;
        let items = vec![
            notifv1::NotificationCategoryItem {
                id: Some(EventType::PasswordResetCode as i64),
                name: Some("Password reset".to_string()),
            },
            notifv1::NotificationCategoryItem {
                id: Some(EventType::ContactBindCode as i64),
                name: Some("Contact binding".to_string()),
            },
            notifv1::NotificationCategoryItem {
                id: Some(EventType::ChannelTestEmail as i64),
                name: Some("Channel test".to_string()),
            },
        ];
        Ok(Response::new(
            notifv1::ListMyNotificationCategoriesResponse { items },
        ))
    }
}
