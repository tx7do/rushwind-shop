//! The file-metadata face (the reference's file_service.go) — the
//! paged CRUD over the `files` table. The bytes never ride here: the
//! object store face (OssService/FileTransferService) owns the
//! content; this is the ledger the metadata queries read.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::files;
use store::paging::fetch_paged;

use shop_proto::proto::pagination::PagingRequest;
use shop_proto::proto::storage::service::v1 as storagev1;

fn file_proto(r: files::Model) -> storagev1::File {
    storagev1::File {
        id: Some(r.id),
        provider: r.provider.as_deref().map(|name| match name {
            "LOCAL" => storagev1::OssProvider::Local,
            "ALIYUN" => storagev1::OssProvider::Aliyun,
            "AWS" => storagev1::OssProvider::Aws,
            "AZURE" => storagev1::OssProvider::Azure,
            "BAIDU" => storagev1::OssProvider::Baidu,
            "QINIU" => storagev1::OssProvider::Qiniu,
            "TENCENT" => storagev1::OssProvider::Tencent,
            "GOOGLE" => storagev1::OssProvider::Google,
            "HUAWEI" => storagev1::OssProvider::Huawei,
            _ => storagev1::OssProvider::Minio,
        } as i32),
        bucket_name: r.bucket_name,
        file_directory: r.file_directory,
        file_guid: r.file_guid,
        save_file_name: r.save_file_name,
        file_name: r.file_name,
        extension: r.extension,
        size: r.size.map(|v| v as u64),
        size_format: r.size_format,
        link_url: r.link_url,
        content_hash: r.content_hash,
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

fn provider_name(v: i32) -> Option<&'static str> {
    Some(match storagev1::OssProvider::try_from(v).ok()? {
        storagev1::OssProvider::Local => "LOCAL",
        storagev1::OssProvider::Aliyun => "ALIYUN",
        storagev1::OssProvider::Aws => "AWS",
        storagev1::OssProvider::Azure => "AZURE",
        storagev1::OssProvider::Baidu => "BAIDU",
        storagev1::OssProvider::Qiniu => "QINIU",
        storagev1::OssProvider::Tencent => "TENCENT",
        storagev1::OssProvider::Google => "GOOGLE",
        storagev1::OssProvider::Huawei => "HUAWEI",
        _ => "MINIO",
    })
}

/// The human spelling of a byte count (the DDL's size_format column).
pub fn size_format_of(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0usize;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.2} {}", UNITS[unit])
    }
}

pub struct FileService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl storagev1::file_service_server::FileService for FileService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<storagev1::ListFileResponse>, Status> {
        let (rows, total) =
            fetch_paged(&self.state.db, files::Entity::find(), &request.into_inner())
                .await
                .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(storagev1::ListFileResponse {
            items: rows.into_iter().map(file_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<storagev1::CountFileResponse>, Status> {
        let (_, total) = fetch_paged(&self.state.db, files::Entity::find(), &request.into_inner())
            .await
            .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(storagev1::CountFileResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<storagev1::GetFileRequest>,
    ) -> Result<Response<storagev1::File>, Status> {
        let req = request.into_inner();
        let Some(storagev1::get_file_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = files::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("file not found"))?;
        Ok(Response::new(file_proto(row)))
    }

    /// The metadata-only create — the caller has already placed the
    /// bytes (the transfer face or a client-side presigned PUT) and
    /// hands the ledger the facts.
    async fn create(
        &self,
        request: Request<storagev1::CreateFileRequest>,
    ) -> Result<Response<storagev1::File>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let row = files::ActiveModel {
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            provider: Set(data.provider.and_then(provider_name).map(String::from)),
            bucket_name: Set(data.bucket_name),
            file_directory: Set(data.file_directory),
            file_guid: Set(data.file_guid),
            save_file_name: Set(data.save_file_name),
            file_name: Set(data.file_name),
            extension: Set(data.extension),
            size: Set(data.size.map(|v| v as i64)),
            size_format: Set(data.size.map(size_format_of)),
            link_url: Set(data.link_url),
            content_hash: Set(data.content_hash),
            created_at: Set(Some(store::now())),
            ..Default::default()
        }
        .insert(&self.state.db)
        .await
        .map_err(db_status)?;
        Ok(Response::new(file_proto(row)))
    }

    async fn update(
        &self,
        request: Request<storagev1::UpdateFileRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(data) = req.data else {
            return Err(bad("data required"));
        };
        let existing = files::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("file not found"))?;
        let mut active: files::ActiveModel = existing.into();
        if let Some(v) = data.file_name {
            active.file_name = Set(Some(v));
        }
        if let Some(v) = data.link_url {
            active.link_url = Set(Some(v));
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
        request: Request<storagev1::DeleteFileRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(storagev1::delete_file_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        files::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}
