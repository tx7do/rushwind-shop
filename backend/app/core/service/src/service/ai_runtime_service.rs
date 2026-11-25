//! The AI runtime faces (chat/content/query) — the execution surfaces
//! that speak to the configured providers. This round they answer
//! Unimplemented: the runtime that drives an OpenAI-compatible
//! endpoint (streaming or not) lands with the AI runtime batch; the
//! data spine these faces read (providers/conversations/messages/
//! usage) is already live.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use shop_proto::proto::ai::service::v1 as aiv1;

const RUNTIME_GAP: &str = "ai runtime (provider calls) not yet implemented";

pub struct AiChatService {
    pub state: Arc<crate::state::AppState>,
}

#[async_trait::async_trait]
impl aiv1::ai_chat_service_server::AiChatService for AiChatService {
    async fn chat(
        &self,
        _request: Request<aiv1::ChatRequest>,
    ) -> Result<Response<tonic::codegen::BoxStream<aiv1::ChatStreamFrame>>, Status> {
        let _ = &self.state;
        Err(Status::unimplemented(RUNTIME_GAP))
    }

    async fn generate(
        &self,
        _request: Request<aiv1::GenerateRequest>,
    ) -> Result<Response<aiv1::GenerateResponse>, Status> {
        Err(Status::unimplemented(RUNTIME_GAP))
    }
}

pub struct AiQueryService {
    pub state: Arc<crate::state::AppState>,
}

#[async_trait::async_trait]
impl aiv1::ai_query_service_server::AiQueryService for AiQueryService {
    async fn ask(
        &self,
        _request: Request<aiv1::AskAiQueryRequest>,
    ) -> Result<Response<aiv1::AskAiQueryResponse>, Status> {
        Err(Status::unimplemented(RUNTIME_GAP))
    }
}

pub struct AiContentService {
    pub state: Arc<crate::state::AppState>,
}

#[async_trait::async_trait]
impl aiv1::ai_content_service_server::AiContentService for AiContentService {
    async fn generate_content(
        &self,
        _request: Request<aiv1::GenerateContentRequest>,
    ) -> Result<Response<aiv1::GenerateContentResponse>, Status> {
        Err(Status::unimplemented(RUNTIME_GAP))
    }

    async fn semantic_search(
        &self,
        _request: Request<aiv1::SemanticSearchRequest>,
    ) -> Result<Response<aiv1::SemanticSearchResponse>, Status> {
        Err(Status::unimplemented(RUNTIME_GAP))
    }

    async fn rebuild_search_index(
        &self,
        _request: Request<pbjson_types::Empty>,
    ) -> Result<Response<aiv1::RebuildSearchIndexResponse>, Status> {
        Err(Status::unimplemented(RUNTIME_GAP))
    }

    async fn get_safety_insights(
        &self,
        _request: Request<aiv1::SafetyInsightsRequest>,
    ) -> Result<Response<aiv1::SafetyInsightsResponse>, Status> {
        Err(Status::unimplemented(RUNTIME_GAP))
    }
}
