//! The file-transfer face — the streaming upload/download over the
//! object store (the reference's file_transfer_service.go): the
//! upload streams collect into the store under a unique object name
//! and land a `files` ledger row; the download streams the object's
//! bytes out in HttpBody chunks. The PUT and POST upload shapes are
//! the same collector — the verbs exist for the HTTP annotation's
//! sake.

use std::sync::Arc;

use tonic::{Request, Response, Status, Streaming};

use sea_orm::{ActiveModelTrait as _, Set};

use crate::service::file_service::size_format_of;
use crate::state::{bad, db_status, AppState};
use store::entities::files;

use shop_proto::proto::google::api::HttpBody;
use shop_proto::proto::storage::service::v1 as storagev1;

/// The chunk the download streams out at (256 KiB — grpc's comfortable
/// default under the 4 MB message ceiling).
const DOWNLOAD_CHUNK: usize = 256 * 1024;

fn oss_missing() -> Status {
    Status::failed_precondition("object storage not configured")
}

pub struct FileTransferService {
    pub state: Arc<AppState>,
}

impl FileTransferService {
    fn engine(&self) -> Result<std::sync::Arc<dyn rushwind_oss::ObjectStorage>, Status> {
        self.state.oss.clone().ok_or_else(oss_missing)
    }

    /// The shared upload collector: drain the stream, store the bytes
    /// under `folder/<uuid>.<ext>`, land the ledger row.
    async fn collect_and_store(
        &self,
        mut stream: Streaming<HttpBody>,
    ) -> Result<(String, Vec<u8>, Option<String>), Status> {
        let engine = self.engine()?;
        let first = stream
            .message()
            .await
            .map_err(|e| Status::internal(format!("recv: {e}")))?
            .ok_or_else(|| bad("empty upload stream"))?;
        let content_type = if first.content_type.is_empty() {
            None
        } else {
            Some(first.content_type.clone())
        };
        let mut bytes = first.data;
        while let Some(chunk) = stream
            .message()
            .await
            .map_err(|e| Status::internal(format!("recv: {e}")))?
        {
            bytes.extend_from_slice(&chunk.data);
        }
        // The first frame's metadata line (if any) may name the file —
        // the reference parses a key line; the wire shape here carries
        // the object facts in the ledger call, so the stream only
        // carries bytes. The object name is always server-minted.
        let extension = content_type
            .as_deref()
            .and_then(|mime| mime.rsplit('/').next())
            .map(|sub| sub.split(';').next().unwrap_or("bin").to_string())
            .unwrap_or_else(|| "bin".to_string());
        let object_name = format!("{}", uuid::Uuid::new_v4().simple());
        let key = format!("upload/{object_name}.{extension}");
        engine
            .put(&key, &bytes, content_type.as_deref())
            .await
            .map_err(|e| Status::internal(format!("put: {e:?}")))?;
        Ok((key, bytes, content_type))
    }

    /// The ledger row the stored object lands (the metadata face's
    /// Create semantics, inlined).
    async fn record_file(
        &self,
        key: &str,
        bytes: &[u8],
        content_type: Option<&str>,
    ) -> Result<files::Model, Status> {
        let (directory, save_name) = match key.rsplit_once('/') {
            Some((dir, name)) => (Some(dir.to_string()), name.to_string()),
            None => (None, key.to_string()),
        };
        let extension = save_name.rsplit('.').next().unwrap_or("bin").to_string();
        let row = files::ActiveModel {
            tenant_id: Set(Some(0)),
            provider: Set(Some("MINIO".to_string())),
            bucket_name: Set(self.state.cfg.oss.as_ref().map(|o| o.bucket.clone())),
            file_directory: Set(directory),
            file_guid: Set(Some(uuid::Uuid::new_v4().simple().to_string())),
            save_file_name: Set(Some(save_name)),
            file_name: Set(None),
            extension: Set(Some(extension)),
            size: Set(Some(bytes.len() as i64)),
            size_format: Set(Some(size_format_of(bytes.len() as u64))),
            link_url: Set(Some(format!("/api/v1/storage/oss/download/{key}"))),
            content_hash: Set(Some(format!("{:x}", md5ish(bytes)))),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        }
        .insert(&self.state.db)
        .await
        .map_err(db_status)?;
        let _ = content_type;
        Ok(row)
    }

    async fn upload(
        &self,
        stream: Streaming<HttpBody>,
    ) -> Result<Response<storagev1::UploadFileResponse>, Status> {
        let (key, bytes, _mime) = self.collect_and_store(stream).await?;
        let row = self.record_file(&key, &bytes, _mime.as_deref()).await?;
        let _ = row;
        Ok(Response::new(storagev1::UploadFileResponse {
            object_name: Some(key),
            presigned_url: None,
        }))
    }
}

/// The FNV-1a digest — a cheap content fingerprint the ledger's
/// content_hash column carries (not a cryptographic checksum; the
/// engine's own integrity rides the S3 etag).
fn md5ish(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[async_trait::async_trait]
impl storagev1::file_transfer_service_server::FileTransferService for FileTransferService {
    async fn download_file(
        &self,
        request: Request<storagev1::DownloadFileRequest>,
    ) -> Result<Response<tonic::codegen::BoxStream<HttpBody>>, Status> {
        let req = request.into_inner();
        let engine = self.engine()?;
        let key = match req.selector {
            Some(storagev1::download_file_request::Selector::DownloadUrl(name))
                if !name.is_empty() =>
            {
                name
            }
            Some(storagev1::download_file_request::Selector::FileId(id)) => {
                use sea_orm::EntityTrait;
                let row = files::Entity::find_by_id(id)
                    .one(&self.state.db)
                    .await
                    .map_err(db_status)?
                    .ok_or_else(|| Status::not_found("file not found"))?;
                let directory = row
                    .file_directory
                    .unwrap_or_default()
                    .trim_matches('/')
                    .to_string();
                let save = row
                    .save_file_name
                    .or(row.file_guid)
                    .ok_or_else(|| Status::not_found("file object name missing"))?;
                if directory.is_empty() {
                    save
                } else {
                    format!("{directory}/{save}")
                }
            }
            _ => return Err(bad("selector required")),
        };
        let bytes = engine.get(&key).await.map_err(|e| match e {
            rushwind_oss::StorageError::NotFound => Status::not_found("object not found"),
            other => Status::internal(format!("get: {other:?}")),
        })?;
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<HttpBody, Status>>(8);
        tokio::spawn(async move {
            let mime = "application/octet-stream".to_string();
            for chunk in bytes.chunks(DOWNLOAD_CHUNK) {
                if tx
                    .send(Ok(HttpBody {
                        content_type: mime.clone(),
                        data: chunk.to_vec(),
                        extensions: Vec::new(),
                    }))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });
        // tonic 的 server-streaming Item 带 Status（the codec's item）
        Ok(Response::new(
            Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))
                as tonic::codegen::BoxStream<HttpBody>,
        ))
    }

    async fn put_upload_file(
        &self,
        request: Request<Streaming<HttpBody>>,
    ) -> Result<Response<storagev1::UploadFileResponse>, Status> {
        self.upload(request.into_inner()).await
    }

    async fn post_upload_file(
        &self,
        request: Request<Streaming<HttpBody>>,
    ) -> Result<Response<storagev1::UploadFileResponse>, Status> {
        self.upload(request.into_inner()).await
    }
}
