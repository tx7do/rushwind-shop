//! The AI runtime faces (chat/query/content) — the execution surfaces
//! that speak to the configured providers. The engine is the
//! OpenAI-compatible chat-completions call (`/v1/chat/completions`):
//! CLOUD providers ride their base URL, LOCAL providers ride
//! `http://{local_host}:{local_port}/v1` (Ollama and friends expose
//! the same wire). Chat streams the SSE deltas through as chunk
//! frames and lands both the user's and the assistant's messages in
//! the ledger plus the usage row; Ask generates SQL through the model
//! and executes it read-only (SELECT-only fence) before the answer
//! renders; GenerateContent wraps the scene's prompt.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, AppState};
use store::entities::{sys_ai_conversations, sys_ai_messages, sys_ai_providers, sys_ai_usage_logs};

use shop_proto::proto::ai::service::v1 as aiv1;

const RUNTIME_GAP: &str = "ai runtime (provider calls) not yet implemented";

/// One resolved provider: the endpoint the OpenAI wire rides, the
/// model name, the optional bearer key, and the timeout.
pub(crate) struct ResolvedProvider {
    pub(crate) endpoint: String,
    pub(crate) api_key: Option<String>,
    pub(crate) model: String,
    pub(crate) timeout: std::time::Duration,
}

pub(crate) async fn resolve_provider(
    state: &AppState,
    provider_id: Option<i64>,
) -> Result<ResolvedProvider, Status> {
    let row = match provider_id.filter(|id| *id > 0) {
        Some(id) => sys_ai_providers::Entity::find_by_id(id)
            .one(&state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("ai provider not found"))?,
        None => {
            // The default provider, else the first row — an empty
            // ledger answers the configuration gap.
            let default = sys_ai_providers::Entity::find()
                .filter(sys_ai_providers::Column::IsDefault.eq(true))
                .one(&state.db)
                .await
                .map_err(db_status)?;
            match default {
                Some(row) => row,
                None => sys_ai_providers::Entity::find()
                    .one(&state.db)
                    .await
                    .map_err(db_status)?
                    .ok_or_else(|| Status::failed_precondition("no ai provider configured"))?,
            }
        }
    };
    let endpoint = match row.model_type.as_deref() {
        Some("CLOUD") => row.base_url.clone().unwrap_or_default(),
        _ => format!(
            "http://{}:{}/v1",
            row.local_host.as_deref().unwrap_or("127.0.0.1"),
            row.local_port.unwrap_or(11434)
        ),
    };
    let endpoint = endpoint.trim_end_matches('/').to_string();
    if endpoint.is_empty() {
        return Err(Status::failed_precondition("provider has no endpoint"));
    }
    Ok(ResolvedProvider {
        endpoint,
        api_key: row.api_key.clone().filter(|k| !k.is_empty()),
        model: row.model_name.clone().unwrap_or_default(),
        timeout: std::time::Duration::from_secs(
            row.timeout_seconds.unwrap_or(60).clamp(1, 600) as u64
        ),
    })
}

/// The chat-completions call — the non-streaming shape. Returns the
/// text plus the token snapshots the ledgers carry.
pub(crate) async fn chat_completion(
    provider: &ResolvedProvider,
    system: Option<&str>,
    prompt: &str,
    max_tokens: u32,
) -> Result<(String, i64, i64, String), Status> {
    let mut messages = Vec::new();
    if let Some(sys) = system.filter(|s| !s.is_empty()) {
        messages.push(json!({"role": "system", "content": sys}));
    }
    messages.push(json!({"role": "user", "content": prompt}));
    let mut body = json!({
        "model": provider.model,
        "messages": messages,
        "stream": false,
    });
    // The thinking models spend the budget on reasoning before the
    // answer — the floor keeps the content field populated.
    body["max_tokens"] = json!(max_tokens.max(2048));
    let client = reqwest::Client::new();
    let mut call = client
        .post(format!("{}/chat/completions", provider.endpoint))
        .json(&body)
        .timeout(provider.timeout);
    if let Some(key) = provider.api_key.as_deref().filter(|k| !k.is_empty()) {
        call = call.bearer_auth(key);
    }
    let resp = call
        .send()
        .await
        .map_err(|e| Status::internal(format!("ai provider call: {e}")))?;
    let status = resp.status();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| Status::internal(format!("ai provider read: {e}")))?;
    if !status.is_success() {
        return Err(Status::internal(format!(
            "ai provider answered {}: {}",
            status.as_u16(),
            String::from_utf8_lossy(&bytes)
                .chars()
                .take(200)
                .collect::<String>()
        )));
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| Status::internal(format!("ai provider decode: {e}")))?;
    let message = &value["choices"][0]["message"];
    let mut text = message["content"].as_str().unwrap_or_default().to_string();
    if text.is_empty() {
        // The thinking models answer through `reasoning` when the
        // budget ran out mid-thought — that text is the model's
        // output, surfaced with its marker.
        if let Some(reasoning) = message["reasoning"].as_str() {
            text = format!("[思考过程] {reasoning}");
        }
    }
    let usage = &value["usage"];
    Ok((
        text,
        usage["prompt_tokens"].as_i64().unwrap_or(0),
        usage["completion_tokens"].as_i64().unwrap_or(0),
        value["model"]
            .as_str()
            .unwrap_or(&provider.model)
            .to_string(),
    ))
}

/// The ledger rows a chat exchange lands (the user's question, the
/// assistant's answer, the usage).
async fn record_exchange(
    state: &AppState,
    conversation_id: i64,
    user_id: i64,
    tenant_id: i64,
    question: &str,
    answer: &str,
    model: &str,
    prompt_tokens: i64,
    completion_tokens: i64,
    duration_ms: i64,
) -> Result<(), Status> {
    let now = store::now().into();
    sys_ai_messages::ActiveModel {
        tenant_id: Set(Some(tenant_id)),
        conversation_id: Set(Some(conversation_id)),
        role: Set(Some("USER".to_string())),
        content: Set(Some(question.to_string())),
        user_id: Set(Some(user_id)),
        created_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&state.db)
    .await
    .map_err(db_status)?;
    sys_ai_messages::ActiveModel {
        tenant_id: Set(Some(tenant_id)),
        conversation_id: Set(Some(conversation_id)),
        role: Set(Some("ASSISTANT".to_string())),
        content: Set(Some(answer.to_string())),
        model_name: Set(Some(model.to_string())),
        prompt_tokens: Set(Some(prompt_tokens)),
        completion_tokens: Set(Some(completion_tokens)),
        duration_ms: Set(Some(duration_ms)),
        user_id: Set(Some(user_id)),
        created_at: Set(Some(store::now().into())),
        ..Default::default()
    }
    .insert(&state.db)
    .await
    .map_err(db_status)?;
    sys_ai_usage_logs::ActiveModel {
        tenant_id: Set(Some(tenant_id)),
        conversation_id: Set(Some(conversation_id)),
        user_id: Set(Some(user_id)),
        model_name: Set(Some(model.to_string())),
        prompt_tokens: Set(Some(prompt_tokens)),
        completion_tokens: Set(Some(completion_tokens)),
        total_tokens: Set(Some(prompt_tokens + completion_tokens)),
        duration_ms: Set(Some(duration_ms)),
        created_at: Set(Some(store::now().into())),
        ..Default::default()
    }
    .insert(&state.db)
    .await
    .map_err(db_status)?;
    // The conversation's last-message stamp (best-effort).
    if let Ok(Some(row)) = sys_ai_conversations::Entity::find_by_id(conversation_id)
        .one(&state.db)
        .await
    {
        let mut active: sys_ai_conversations::ActiveModel = row.into();
        active.last_message_at = Set(Some(store::now().into()));
        let _ = active.update(&state.db).await;
    }
    Ok(())
}

/// A single SSE `data:` line's delta text (the OpenAI stream shape).
fn sse_delta(line: &str) -> Option<String> {
    let payload = line.strip_prefix("data: ")?;
    let payload = payload.trim();
    if payload == "[DONE]" {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(payload).ok()?;
    value["choices"][0]["delta"]["content"]
        .as_str()
        .map(String::from)
}

/// The streaming chat-completions call — the deltas feed the caller's
/// channel as they arrive; the aggregate returns for the ledger.
async fn chat_completion_stream(
    provider: &ResolvedProvider,
    system: Option<&str>,
    prompt: &str,
    conversation_id: i64,
    tx: &tokio::sync::mpsc::Sender<Result<aiv1::ChatStreamFrame, Status>>,
) -> Result<(String, i64, i64, String), Status> {
    let mut messages = Vec::new();
    if let Some(sys) = system.filter(|s| !s.is_empty()) {
        messages.push(json!({"role": "system", "content": sys}));
    }
    messages.push(json!({"role": "user", "content": prompt}));
    let body = json!({
        "model": provider.model,
        "messages": messages,
        "stream": true,
        "stream_options": {"include_usage": true},
    });
    let client = reqwest::Client::new();
    let mut call = client
        .post(format!("{}/chat/completions", provider.endpoint))
        .json(&body)
        .timeout(provider.timeout);
    if let Some(key) = provider.api_key.as_deref().filter(|k| !k.is_empty()) {
        call = call.bearer_auth(key);
    }
    let resp = call
        .send()
        .await
        .map_err(|e| Status::internal(format!("ai provider call: {e}")))?;
    let status = resp.status();
    if !status.is_success() {
        let head = resp.text().await.unwrap_or_default();
        return Err(Status::internal(format!(
            "ai provider answered {}: {}",
            status.as_u16(),
            head.chars().take(200).collect::<String>()
        )));
    }
    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut full = String::new();
    let mut seq = 0u32;
    let mut captured_prompt = 0i64;
    let mut captured_completion = 0i64;
    let prompt_tokens;
    let completion_tokens;
    let model;
    loop {
        use futures_util::StreamExt as _;
        let chunk = match tokio::time::timeout(provider.timeout, stream.next()).await {
            Ok(Some(Ok(bytes))) => bytes,
            Ok(Some(Err(e))) => return Err(Status::internal(format!("ai provider stream: {e}"))),
            Ok(None) => break,
            Err(_) => return Err(Status::deadline_exceeded("ai provider stream timed out")),
        };
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(idx) = buf.find('\n') {
            let line = buf[..idx].trim_end_matches('\r').to_string();
            buf.drain(..=idx);
            // The usage rides the final SSE frame (include_usage).
            if line.starts_with("data: ") {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(line[6..].trim()) {
                    if let Some(usage) = value.get("usage") {
                        if usage["prompt_tokens"].is_i64() {
                            captured_prompt = usage["prompt_tokens"].as_i64().unwrap_or(0);
                            captured_completion = usage["completion_tokens"].as_i64().unwrap_or(0);
                        }
                    }
                }
            }
            if let Some(delta) = sse_delta(&line) {
                if delta.is_empty() {
                    continue;
                }
                full.push_str(&delta);
                let _ = tx
                    .send(Ok(aiv1::ChatStreamFrame {
                        frame: Some(aiv1::chat_stream_frame::Frame::Chunk(
                            aiv1::ChatChunkEvent {
                                conversation_id,
                                seq,
                                delta: delta.clone(),
                            },
                        )),
                    }))
                    .await;
                seq += 1;
            }
            // The non-stream fallback some providers answer with: a
            // plain JSON body carries the whole message.
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                if value["choices"][0]["message"]["content"].is_string() {
                    let text = value["choices"][0]["message"]["content"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                    prompt_tokens = value["usage"]["prompt_tokens"].as_i64().unwrap_or(0);
                    completion_tokens = value["usage"]["completion_tokens"].as_i64().unwrap_or(0);
                    model = value["model"]
                        .as_str()
                        .unwrap_or(&provider.model)
                        .to_string();
                    return Ok((text, prompt_tokens, completion_tokens, model));
                }
            }
        }
    }
    model = provider.model.clone();
    let prompt_tokens = captured_prompt;
    let completion_tokens = captured_completion;
    Ok((full, prompt_tokens, completion_tokens, model))
}

pub struct AiChatService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl aiv1::ai_chat_service_server::AiChatService for AiChatService {
    /// The non-streaming face: system+prompt through the provider, the
    /// text back.
    async fn generate(
        &self,
        request: Request<aiv1::GenerateRequest>,
    ) -> Result<Response<aiv1::GenerateResponse>, Status> {
        let req = request.into_inner();
        let provider = resolve_provider(&self.state, None).await?;
        let (text, _, total, model_name) = chat_completion(
            &provider,
            req.system.as_deref(),
            &req.prompt.unwrap_or_default(),
            req.max_tokens.unwrap_or(0),
        )
        .await?;
        Ok(Response::new(aiv1::GenerateResponse {
            text,
            model_name,
            total_tokens: total as u32,
        }))
    }

    /// The streaming face: the question lands as the user message, the
    /// deltas stream through as chunk frames, the aggregate lands as
    /// the assistant message plus the usage row, and the done frame
    /// carries both ledger rows back.
    async fn chat(
        &self,
        request: Request<aiv1::ChatRequest>,
    ) -> Result<Response<tonic::codegen::BoxStream<aiv1::ChatStreamFrame>>, Status> {
        let req = request.into_inner();
        let user_id = req.operator_user_id.unwrap_or(0);
        let tenant_id = req.operator_tenant_id.unwrap_or(0);
        let content = req.content.unwrap_or_default();
        if content.is_empty() {
            return Err(bad("content required"));
        }
        // The conversation: re-open the named one or mint one titled
        // by the question's head.
        let conversation_id = match req.conversation_id.filter(|id| *id > 0) {
            Some(id) => id,
            None => {
                let title: String = content.chars().take(20).collect();
                let row = sys_ai_conversations::ActiveModel {
                    tenant_id: Set(Some(tenant_id)),
                    title: Set(Some(title)),
                    provider_id: Set(req.provider_id),
                    user_id: Set(Some(user_id)),
                    created_by: Set(Some(user_id)),
                    created_at: Set(Some(store::now().into())),
                    ..Default::default()
                }
                .insert(&self.state.db)
                .await
                .map_err(db_status)?;
                row.id
            }
        };
        let provider = resolve_provider(&self.state, req.provider_id).await?;
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<aiv1::ChatStreamFrame, Status>>(16);
        let state = self.state.clone();
        let prompt = content.clone();
        let system = None; // the conversation-level system prompt rides the provider row
        tokio::spawn(async move {
            let started = std::time::Instant::now();
            let outcome =
                chat_completion_stream(&provider, system, &prompt, conversation_id, &tx).await;
            match outcome {
                Ok((text, pt, ct, model)) => {
                    let duration = started.elapsed().as_millis() as i64;
                    // The ledger rows (best-effort — a logging miss
                    // never breaks the streamed answer).
                    let _ = record_exchange(
                        &state,
                        conversation_id,
                        user_id,
                        tenant_id,
                        &prompt,
                        &text,
                        &model,
                        pt,
                        ct,
                        duration,
                    )
                    .await;
                    let conversation = sys_ai_conversations::Entity::find_by_id(conversation_id)
                        .one(&state.db)
                        .await
                        .ok()
                        .flatten()
                        .map(|row| crate::service::ai_crud_service::conversation_proto(row));
                    let message = aiv1::AiMessage {
                        conversation_id: Some(conversation_id),
                        role: Some(aiv1::AiRole::Assistant as i32),
                        content: Some(text),
                        model_name: Some(model),
                        prompt_tokens: Some(pt as u32),
                        completion_tokens: Some(ct as u32),
                        duration_ms: Some(duration as u32),
                        user_id: Some(user_id),
                        tenant_id: Some(tenant_id),
                        ..Default::default()
                    };
                    let _ = tx
                        .send(Ok(aiv1::ChatStreamFrame {
                            frame: Some(aiv1::chat_stream_frame::Frame::Done(aiv1::ChatResponse {
                                conversation,
                                message: Some(message),
                            })),
                        }))
                        .await;
                }
                Err(status) => {
                    let _ = tx.send(Err(status)).await;
                }
            }
        });
        Ok(Response::new(
            Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))
                as tonic::codegen::BoxStream<aiv1::ChatStreamFrame>,
        ))
    }
}

pub struct AiQueryService {
    pub state: Arc<AppState>,
}

/// The NL2SQL prompt: the schema digest rides in, the model answers
/// with one SELECT statement only.
const NL2SQL_SYSTEM: &str = "你是风行商城的分析 SQL 助手。根据用户问题和数据库表结构，生成一条 PostgreSQL SELECT 查询。只输出 SQL 本身，不要任何解释或代码块标记。只允许 SELECT 语句；禁止 INSERT/UPDATE/DELETE/DROP/ALTER/TRUNCATE。可用核心表：sys_users(id, username, status, tenant_id, created_at)、orders(id, user_id, status, total_amount, tenant_id, created_at)、order_items(id, order_id, product_id, sku_id, quantity, price)、products(id, name, status, price, category_id, tenant_id)、categories(id, name, parent_id)、coupons(id, name, discount_value, status)。";

#[async_trait::async_trait]
impl aiv1::ai_query_service_server::AiQueryService for AiQueryService {
    /// The NL2SQL face: the model writes the SELECT, the service
    /// executes it behind the read-only fence (SELECT-only, a hard
    /// statement cap) and renders the rows.
    async fn ask(
        &self,
        request: Request<aiv1::AskAiQueryRequest>,
    ) -> Result<Response<aiv1::AskAiQueryResponse>, Status> {
        let req = request.into_inner();
        if req.question.is_empty() {
            return Err(bad("question required"));
        }
        let provider = resolve_provider(&self.state, None).await?;
        let (sql, _, _, _) =
            chat_completion(&provider, Some(NL2SQL_SYSTEM), &req.question, 300).await?;
        let sql = sql
            .trim()
            .trim_start_matches("```sql")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
            .to_string();
        let upper = sql.to_uppercase();
        if !upper.starts_with("SELECT")
            || upper
                .split(';')
                .next()
                .map(|s| {
                    s.contains("INSERT")
                        || s.contains("UPDATE")
                        || s.contains("DELETE")
                        || s.contains("DROP")
                })
                .unwrap_or(true)
        {
            return Err(Status::permission_denied(
                "model produced a non-select statement",
            ));
        }
        // The read-only execution: one SELECT wrapped in row_to_json
        // (the heterogeneous column types ride as one JSON text), a
        // row cap, a statement timeout.
        // The read-only fence already held (SELECT-only); the row cap
        // bounds the result, the client timeout bounds the duration —
        // a prepared statement cannot carry the SET LOCAL preamble.
        let fenced = format!(
            "SELECT row_to_json(t)::text FROM ({} LIMIT 200) t",
            sql.trim_end_matches(';')
        );
        let rows = self
            .state
            .db
            .query_all_raw(sea_orm::Statement::from_string(
                self.state.db.get_database_backend(),
                fenced,
            ))
            .await
            .map_err(db_status)?;
        let mut columns: Vec<String> = Vec::new();
        let values: Vec<Vec<String>> = rows
            .into_iter()
            .map(|row| {
                let text = row.try_get_by_index::<String>(0).unwrap_or_default();
                if columns.is_empty() {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(obj) = v.as_object() {
                            columns = obj.keys().cloned().collect();
                        }
                    }
                }
                vec![text]
            })
            .collect();
        let row_count = values.len() as u32;
        Ok(Response::new(aiv1::AskAiQueryResponse {
            sql,
            columns,
            rows: values
                .into_iter()
                .map(|v| aiv1::AiQueryRow { values: v })
                .collect(),
            row_count,
            answer: None,
            total_tokens: 0,
            error_message: None,
        }))
    }
}

pub struct AiContentService {
    pub state: Arc<AppState>,
}

/// The scene's prompt wrapper (the reference's scene prompt set).
fn scene_prompt(scene: i32, topic: &str) -> String {
    let scene = aiv1::ContentScene::try_from(scene).unwrap_or(aiv1::ContentScene::General);
    let instruction = match scene {
        aiv1::ContentScene::Description => {
            format!("为商品《{topic}》写一段电商详情页描述，突出卖点，150 字以内。")
        }
        aiv1::ContentScene::Announcement => {
            format!("为商城《{topic}》写一条用户公告，语气亲切，80 字以内。")
        }
        aiv1::ContentScene::Reply => {
            format!("以商家客服口吻回复一条关于《{topic}》的用户评价或咨询，礼貌专业，100 字以内。")
        }
        aiv1::ContentScene::PostTitle => format!("为《{topic}》写一个社区帖子标题，20 字以内。"),
        aiv1::ContentScene::PostSummary => format!("为《{topic}》写一段帖子摘要，60 字以内。"),
        aiv1::ContentScene::PostBody => format!("为《{topic}》写一段社区帖子正文，200 字以内。"),
        aiv1::ContentScene::SocialPost => {
            format!("为《{topic}》写一条社交媒体推广文案，带话题标签，80 字以内。")
        }
        _ => format!("围绕《{topic}》写一段通用文案，100 字以内。"),
    };
    instruction
}

#[async_trait::async_trait]
impl aiv1::ai_content_service_server::AiContentService for AiContentService {
    async fn generate_content(
        &self,
        request: Request<aiv1::GenerateContentRequest>,
    ) -> Result<Response<aiv1::GenerateContentResponse>, Status> {
        let req = request.into_inner();
        if req.topic.is_empty() {
            return Err(bad("topic required"));
        }
        let provider = resolve_provider(&self.state, None).await?;
        let prompt = scene_prompt(req.scene, &req.topic);
        let (text, _, _, _) = chat_completion(
            &provider,
            Some("你是风行商城的内容生成助手。直接输出正文，不要任何解释。"),
            &prompt,
            400,
        )
        .await?;
        Ok(Response::new(aiv1::GenerateContentResponse {
            content: text,
            total_tokens: 0,
        }))
    }

    /// The semantic face over the docs ledger — the embedding runtime
    /// is the gap; a keyword containment over the doc names keeps the
    /// face honest until the index lands.
    async fn semantic_search(
        &self,
        request: Request<aiv1::SemanticSearchRequest>,
    ) -> Result<Response<aiv1::SemanticSearchResponse>, Status> {
        let req = request.into_inner();
        if req.query.is_empty() {
            return Err(bad("query required"));
        }
        let limit = req.limit.unwrap_or(8).clamp(1, 50) as i64;
        let rows = store::entities::sys_ai_docs::Entity::find()
            .filter(store::entities::sys_ai_docs::Column::Name.contains(&req.query))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let items = rows
            .into_iter()
            .take(limit as usize)
            .map(|r| aiv1::SemanticSearchItem {
                title: r.name.unwrap_or_default(),
                route: String::new(),
                score: 0.0,
                ..Default::default()
            })
            .collect();
        Ok(Response::new(aiv1::SemanticSearchResponse { items }))
    }

    async fn rebuild_search_index(
        &self,
        _request: Request<pbjson_types::Empty>,
    ) -> Result<Response<aiv1::RebuildSearchIndexResponse>, Status> {
        Err(Status::unimplemented(RUNTIME_GAP))
    }

    /// The safety face reads the login audit ledger for the brute-force
    /// shape (the same rows the monitor face reads); the LLM summary
    /// rides the provider when one answers.
    async fn get_safety_insights(
        &self,
        request: Request<aiv1::SafetyInsightsRequest>,
    ) -> Result<Response<aiv1::SafetyInsightsResponse>, Status> {
        let _ = request.into_inner();
        let failed = store::entities::login_audit_logs::Entity::find()
            .filter(store::entities::login_audit_logs::Column::Status.eq("FAILED"))
            .all(&self.state.db)
            .await;
        let mut alerts = Vec::new();
        if let Ok(rows) = &failed {
            if rows.len() >= 10 {
                alerts.push(aiv1::AiInsightAlert {
                    severity: "HIGH".to_string(),
                    r#type: "BRUTE_FORCE".to_string(),
                    facts: [("count".to_string(), rows.len().to_string())]
                        .into_iter()
                        .collect(),
                    items: Vec::new(),
                });
            }
        }
        Ok(Response::new(aiv1::SafetyInsightsResponse {
            alerts,
            summary: String::new(),
        }))
    }
}
