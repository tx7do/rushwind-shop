//! The AI management faces (the reference's ai_provider/conversation/
//! message/usage_log services) — the four CRUD ledgers the AI
//! runtime reads: the OpenAI-compatible provider configs (the API key
//! never rides the wire back — only the masked hint), the per-user
//! conversations, their messages, and the token-usage ledger with the
//! monthly summary. The interactive faces (chat/agent/query) land
//! with the AI runtime; these faces are the data spine.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{sys_ai_conversations, sys_ai_messages, sys_ai_providers, sys_ai_usage_logs};
use store::paging::fetch_paged;

use shop_proto::proto::ai::service::v1 as aiv1;
use shop_proto::proto::pagination::PagingRequest;

// ── Provider ─────────────────────────────────────────────────────────

fn model_type_name(v: i32) -> Option<&'static str> {
    Some(match aiv1::ai_provider::ModelType::try_from(v).ok()? {
        aiv1::ai_provider::ModelType::Cloud => "CLOUD",
        _ => "LOCAL",
    })
}

fn model_type_proto(name: Option<&str>) -> Option<i32> {
    Some(match name? {
        "CLOUD" => aiv1::ai_provider::ModelType::Cloud as i32,
        _ => aiv1::ai_provider::ModelType::Local as i32,
    })
}

fn provider_proto(r: sys_ai_providers::Model) -> aiv1::AiProvider {
    aiv1::AiProvider {
        id: Some(r.id),
        name: r.name,
        model_type: model_type_proto(r.model_type.as_deref()),
        model_name: r.model_name,
        base_url: r.base_url,
        organization: r.organization,
        api_key: None, // the key never rides back — the masked hint does
        api_key_hint: r.api_key_hint,
        local_host: r.local_host,
        local_port: r.local_port,
        timeout_seconds: r.timeout_seconds,
        system_prompt: r.system_prompt,
        is_default: r.is_default,
        remark: r.remark,
        is_enabled: r.is_enabled,
        created_by: r.created_by,
        updated_by: r.updated_by,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

async fn provider_by_id(
    db: &sea_orm::DatabaseConnection,
    id: i64,
) -> Result<sys_ai_providers::Model, Status> {
    sys_ai_providers::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(db_status)?
        .ok_or_else(|| Status::not_found("ai provider not found"))
}

/// The masked hint the wire carries wherever a key lands
/// (`sk-***abcd`, the reference's shape).
fn key_hint_of(key: &str) -> String {
    let trimmed = key.trim();
    let visible = 4usize.min(trimmed.len());
    let head = trimmed.chars().take(3).collect::<String>();
    let tail = trimmed
        .chars()
        .skip(trimmed.len() - visible)
        .collect::<String>();
    format!("{head}***{tail}")
}

pub struct AiProviderService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl aiv1::ai_provider_service_server::AiProviderService for AiProviderService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<aiv1::ListAiProviderResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sys_ai_providers::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(aiv1::ListAiProviderResponse {
            items: rows.into_iter().map(provider_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<aiv1::CountAiProviderResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            sys_ai_providers::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(aiv1::CountAiProviderResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<aiv1::GetAiProviderRequest>,
    ) -> Result<Response<aiv1::AiProvider>, Status> {
        let req = request.into_inner();
        let Some(aiv1::get_ai_provider_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        Ok(Response::new(provider_proto(
            provider_by_id(&self.state.db, id).await?,
        )))
    }

    async fn create(
        &self,
        request: Request<aiv1::CreateAiProviderRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let api_key = data.api_key.clone().filter(|k| !k.is_empty());
        let active = sys_ai_providers::ActiveModel {
            tenant_id: Set(Some(0)),
            name: Set(data.name.clone()),
            model_type: Set(data.model_type.and_then(model_type_name).map(String::from)),
            model_name: Set(data.model_name),
            base_url: Set(data.base_url),
            organization: Set(data.organization),
            api_key: Set(api_key.clone()),
            api_key_hint: Set(api_key.as_deref().map(key_hint_of)),
            local_host: Set(data.local_host),
            local_port: Set(data.local_port),
            timeout_seconds: Set(data.timeout_seconds),
            system_prompt: Set(data.system_prompt),
            is_default: Set(data.is_default),
            remark: Set(data.remark),
            created_by: Set(data.created_by),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<aiv1::UpdateAiProviderRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let existing = provider_by_id(&self.state.db, req.id).await?;
        let mut active: sys_ai_providers::ActiveModel = existing.into();
        if let Some(v) = data.name {
            active.name = Set(Some(v));
        }
        if let Some(v) = data.model_type {
            active.model_type = Set(model_type_name(v).map(String::from));
        }
        if let Some(v) = data.model_name {
            active.model_name = Set(Some(v));
        }
        if let Some(v) = data.base_url {
            active.base_url = Set(Some(v));
        }
        if let Some(v) = data.organization {
            active.organization = Set(Some(v));
        }
        if let Some(v) = data.api_key.clone().filter(|k| !k.is_empty()) {
            active.api_key = Set(Some(v.clone()));
            active.api_key_hint = Set(Some(key_hint_of(&v)));
        }
        if let Some(v) = data.local_host {
            active.local_host = Set(Some(v));
        }
        if let Some(v) = data.local_port {
            active.local_port = Set(Some(v));
        }
        if let Some(v) = data.timeout_seconds {
            active.timeout_seconds = Set(Some(v));
        }
        if let Some(v) = data.system_prompt {
            active.system_prompt = Set(Some(v));
        }
        if let Some(v) = data.is_default {
            active.is_default = Set(Some(v));
        }
        if let Some(v) = data.remark {
            active.remark = Set(Some(v));
        }
        if let Some(v) = data.updated_by {
            active.updated_by = Set(Some(v));
        }
        active.updated_at = Set(Some(store::now()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<aiv1::DeleteAiProviderRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(aiv1::delete_ai_provider_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        sys_ai_providers::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

// ── Conversation ─────────────────────────────────────────────────────

pub fn conversation_proto(r: sys_ai_conversations::Model) -> aiv1::AiConversation {
    aiv1::AiConversation {
        id: Some(r.id),
        title: r.title,
        provider_id: r.provider_id,
        user_id: r.user_id,
        tenant_id: r.tenant_id,
        last_message_at: r.last_message_at.and_then(ts_to_proto),
        created_by: r.created_by,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

pub struct AiConversationService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl aiv1::ai_conversation_service_server::AiConversationService for AiConversationService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<aiv1::ListAiConversationResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sys_ai_conversations::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(aiv1::ListAiConversationResponse {
            items: rows.into_iter().map(conversation_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<aiv1::CountAiConversationResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            sys_ai_conversations::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(aiv1::CountAiConversationResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<aiv1::GetAiConversationRequest>,
    ) -> Result<Response<aiv1::AiConversation>, Status> {
        let req = request.into_inner();
        let Some(aiv1::get_ai_conversation_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = sys_ai_conversations::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("ai conversation not found"))?;
        Ok(Response::new(conversation_proto(row)))
    }

    async fn update(
        &self,
        request: Request<aiv1::UpdateAiConversationRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let existing = sys_ai_conversations::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("ai conversation not found"))?;
        let mut active: sys_ai_conversations::ActiveModel = existing.into();
        if let Some(v) = data.title {
            active.title = Set(Some(v));
        }
        if let Some(v) = data.provider_id {
            active.provider_id = Set(Some(v));
        }
        if let Some(v) = data.updated_by {
            active.updated_by = Set(Some(v));
        }
        active.updated_at = Set(Some(store::now()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The cascade: the conversation's messages go with it.
    async fn delete(
        &self,
        request: Request<aiv1::DeleteAiConversationRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(aiv1::delete_ai_conversation_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        sys_ai_messages::Entity::delete_many()
            .filter(sys_ai_messages::Column::ConversationId.eq(id))
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        sys_ai_conversations::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

// ── Message ──────────────────────────────────────────────────────────

fn role_proto(name: Option<&str>) -> Option<i32> {
    Some(match name? {
        "ASSISTANT" => aiv1::AiRole::Assistant as i32,
        "SYSTEM" => aiv1::AiRole::System as i32,
        _ => aiv1::AiRole::User as i32,
    })
}

fn message_proto(r: sys_ai_messages::Model) -> aiv1::AiMessage {
    aiv1::AiMessage {
        id: Some(r.id),
        conversation_id: r.conversation_id,
        role: role_proto(r.role.as_deref()),
        content: r.content,
        model_name: r.model_name,
        prompt_tokens: r.prompt_tokens.map(|v| v as u32),
        completion_tokens: r.completion_tokens.map(|v| v as u32),
        duration_ms: r.duration_ms.map(|v| v as u32),
        error_message: r.error_message,
        user_id: r.user_id,
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

pub struct AiMessageService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl aiv1::ai_message_service_server::AiMessageService for AiMessageService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<aiv1::ListAiMessageResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sys_ai_messages::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(aiv1::ListAiMessageResponse {
            items: rows.into_iter().map(message_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<aiv1::CountAiMessageResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            sys_ai_messages::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(aiv1::CountAiMessageResponse {
            count: total as i64,
        }))
    }

    async fn delete(
        &self,
        request: Request<aiv1::DeleteAiMessageRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(aiv1::delete_ai_message_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        sys_ai_messages::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}

// ── UsageLog ─────────────────────────────────────────────────────────

fn usage_proto(r: sys_ai_usage_logs::Model) -> aiv1::AiUsageLog {
    aiv1::AiUsageLog {
        id: Some(r.id),
        provider_id: r.provider_id,
        conversation_id: r.conversation_id,
        user_id: r.user_id,
        tenant_id: r.tenant_id,
        model_name: r.model_name,
        prompt_tokens: r.prompt_tokens.map(|v| v as u32),
        completion_tokens: r.completion_tokens.map(|v| v as u32),
        total_tokens: r.total_tokens.map(|v| v as u32),
        duration_ms: r.duration_ms.map(|v| v as u32),
        created_at: r.created_at.and_then(ts_to_proto),
    }
}

pub struct AiUsageLogService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl aiv1::ai_usage_log_service_server::AiUsageLogService for AiUsageLogService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<aiv1::ListAiUsageLogResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sys_ai_usage_logs::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(aiv1::ListAiUsageLogResponse {
            items: rows.into_iter().map(usage_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<aiv1::CountAiUsageLogResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            sys_ai_usage_logs::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(aiv1::CountAiUsageLogResponse {
            count: total as i64,
        }))
    }

    /// The caller's month: token/call totals over the calendar month,
    /// the plan's AI_TOKENS quota attached when the tenant rides one.
    async fn get_usage_summary(
        &self,
        request: Request<pbjson_types::Empty>,
    ) -> Result<Response<aiv1::UsageSummaryResponse>, Status> {
        let _ = request.into_inner();
        use chrono::Datelike;
        let today = store::now().date_naive();
        let first = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
            .expect("first of month");
        let month_start = first
            .and_hms_opt(0, 0, 0)
            .expect("midnight")
            .and_local_timezone(chrono::Utc)
            .single()
            .expect("unambiguous midnight")
            .fixed_offset();
        let rows = sys_ai_usage_logs::Entity::find()
            .filter(sys_ai_usage_logs::Column::CreatedAt.gte(month_start))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let month_tokens: i64 = rows.iter().map(|r| r.total_tokens.unwrap_or(0)).sum();
        let quota = store::entities::sys_plan_quotas::Entity::find()
            .filter(store::entities::sys_plan_quotas::Column::QuotaType.eq("AI_TOKENS"))
            .one(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(aiv1::UsageSummaryResponse {
            month_tokens: month_tokens as u32,
            month_calls: rows.len() as u32,
            quota_configured: quota.is_some(),
            quota_limit: quota.and_then(|q| q.quota_value).unwrap_or(0) as u64,
            ..Default::default()
        }))
    }
}
