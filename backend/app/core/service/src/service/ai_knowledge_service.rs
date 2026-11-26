//! The knowledge-base face (the reference's ai_knowledge_service.go)
//! — the base CRUD, the doc ledger (UploadDoc registers the metadata
//! and answers READY: the embedding pipeline is the AI runtime's), and
//! the doc listing/removal. The vector faces (Search/ReindexDocs and
//! the file-body UploadDocFile) answer Unimplemented — they land with
//! the embedding runtime (the same gap the chat/content faces carry).

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{sys_ai_docs, sys_ai_knowledge_bases};
use store::paging::fetch_paged;

use shop_proto::proto::ai::service::v1 as aiv1;
use shop_proto::proto::pagination::PagingRequest;

fn base_proto(r: sys_ai_knowledge_bases::Model) -> aiv1::AiKnowledgeBase {
    aiv1::AiKnowledgeBase {
        id: Some(r.id),
        name: r.name,
        description: r.description,
        provider_id: r.provider_id,
        embedding_model: r.embedding_model,
        user_id: r.user_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

fn doc_proto(r: sys_ai_docs::Model) -> aiv1::AiDoc {
    aiv1::AiDoc {
        id: Some(r.id),
        base_id: r.base_id,
        name: r.name,
        status: r.status,
        error_message: r.error_message,
        user_id: r.user_id,
        created_at: r.created_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

async fn base_by_id(
    db: &sea_orm::DatabaseConnection,
    id: i64,
) -> Result<sys_ai_knowledge_bases::Model, Status> {
    sys_ai_knowledge_bases::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(db_status)?
        .ok_or_else(|| Status::not_found("knowledge base not found"))
}

pub struct AiKnowledgeBaseService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl aiv1::ai_knowledge_base_service_server::AiKnowledgeBaseService for AiKnowledgeBaseService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<aiv1::ListAiKnowledgeBaseResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            sys_ai_knowledge_bases::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(aiv1::ListAiKnowledgeBaseResponse {
            items: rows.into_iter().map(base_proto).collect(),
            total: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<aiv1::GetAiKnowledgeBaseRequest>,
    ) -> Result<Response<aiv1::AiKnowledgeBase>, Status> {
        let req = request.into_inner();
        let Some(aiv1::get_ai_knowledge_base_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        Ok(Response::new(base_proto(
            base_by_id(&self.state.db, id).await?,
        )))
    }

    async fn create(
        &self,
        request: Request<aiv1::CreateAiKnowledgeBaseRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let active = sys_ai_knowledge_bases::ActiveModel {
            tenant_id: Set(Some(0)),
            name: Set(data.name),
            description: Set(data.description),
            provider_id: Set(data.provider_id),
            embedding_model: Set(data.embedding_model),
            user_id: Set(data.user_id),
            created_by: Set(data.user_id),
            created_at: Set(Some(store::now())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<aiv1::UpdateAiKnowledgeBaseRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let existing = base_by_id(&self.state.db, req.id).await?;
        let mut active: sys_ai_knowledge_bases::ActiveModel = existing.into();
        if let Some(v) = data.name {
            active.name = Set(Some(v));
        }
        if let Some(v) = data.description {
            active.description = Set(Some(v));
        }
        if let Some(v) = data.provider_id {
            active.provider_id = Set(Some(v));
        }
        if let Some(v) = data.embedding_model {
            active.embedding_model = Set(Some(v));
        }
        active.updated_at = Set(Some(store::now()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The cascade: the base's docs go with it.
    async fn delete(
        &self,
        request: Request<aiv1::DeleteAiKnowledgeBaseRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(aiv1::delete_ai_knowledge_base_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        sys_ai_docs::Entity::delete_many()
            .filter(sys_ai_docs::Column::BaseId.eq(id))
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        sys_ai_knowledge_bases::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The metadata registration: the doc lands READY this round (the
    /// embedding index is the runtime's).
    async fn upload_doc(
        &self,
        request: Request<aiv1::UploadAiDocRequest>,
    ) -> Result<Response<aiv1::UploadAiDocResponse>, Status> {
        let req = request.into_inner();
        let base_id = req.base_id;
        base_by_id(&self.state.db, base_id).await?;
        let row = sys_ai_docs::ActiveModel {
            tenant_id: Set(Some(0)),
            base_id: Set(Some(base_id)),
            name: Set(Some(req.name)),
            status: Set(Some("READY".to_string())),
            user_id: Set(None),
            created_at: Set(Some(store::now())),
            ..Default::default()
        }
        .insert(&self.state.db)
        .await
        .map_err(db_status)?;
        Ok(Response::new(aiv1::UploadAiDocResponse {
            doc: Some(doc_proto(row)),
            chunk_count: 0,
        }))
    }

    async fn upload_doc_file(
        &self,
        _request: Request<aiv1::UploadAiDocFileRequest>,
    ) -> Result<Response<aiv1::UploadAiDocResponse>, Status> {
        Err(Status::unimplemented(
            "doc-file ingestion rides the embedding runtime",
        ))
    }

    async fn list_docs(
        &self,
        request: Request<aiv1::ListAiDocsRequest>,
    ) -> Result<Response<aiv1::ListAiDocsResponse>, Status> {
        let req = request.into_inner();
        let base_id = req.base_id;
        base_by_id(&self.state.db, base_id).await?;
        let rows = sys_ai_docs::Entity::find()
            .filter(sys_ai_docs::Column::BaseId.eq(base_id))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let items: Vec<aiv1::AiDoc> = rows.into_iter().map(doc_proto).collect();
        let total = items.len() as i64;
        Ok(Response::new(aiv1::ListAiDocsResponse { items, total }))
    }

    async fn delete_doc(
        &self,
        request: Request<aiv1::DeleteAiDocRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        sys_ai_docs::Entity::delete_by_id(req.id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn search(
        &self,
        _request: Request<aiv1::SearchAiKnowledgeRequest>,
    ) -> Result<Response<aiv1::SearchAiKnowledgeResponse>, Status> {
        Err(Status::unimplemented(
            "semantic search rides the embedding runtime",
        ))
    }

    async fn reindex_docs(
        &self,
        _request: Request<aiv1::ReindexDocsRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        Err(Status::unimplemented("reindex rides the embedding runtime"))
    }
}
