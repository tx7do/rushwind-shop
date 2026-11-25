//! The OSS face (the reference's oss endpoints through the engine
//! crate) — presigned upload/download grants, the object listing, and
//! the delete. The engine is the config-selected ObjectStorage for the
//! default bucket; a request naming a DIFFERENT bucket gets an engine
//! built for that bucket on demand (same endpoint/credentials), cached
//! per service instance — real multi-bucket over the single-bucket
//! config shape.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::state::{bad, AppState};

use shop_proto::proto::storage::service::v1 as storagev1;

/// The storage faces' not-configured answer.
fn oss_missing() -> Status {
    Status::failed_precondition("object storage not configured")
}

/// A 32-hex object stem (the same random draw class as the refresh
/// token — no client-controlled name ever lands in the bucket).
fn unique_object_name(extension: &str) -> String {
    let stem = uuid::Uuid::new_v4().simple().to_string();
    let ext = extension.trim_start_matches('.');
    if ext.is_empty() {
        stem
    } else {
        format!("{stem}.{ext}")
    }
}

pub struct OssService {
    pub state: Arc<AppState>,
    /// Engines built on demand for non-default buckets (the config
    /// engine lives in AppState.oss; this cache grows per requested
    /// bucket name).
    pub extra_engines:
        std::sync::Mutex<std::collections::HashMap<String, Arc<dyn rushwind_oss::ObjectStorage>>>,
}

impl OssService {
    fn settings(&self) -> Result<&crate::config::OssSettings, Status> {
        self.state.cfg.oss.as_ref().ok_or_else(oss_missing)
    }

    /// The engine for the request's bucket: the configured one for the
    /// default/absent name, an on-demand cached engine otherwise.
    async fn engine_for(
        &self,
        bucket: Option<&str>,
    ) -> Result<(std::sync::Arc<dyn rushwind_oss::ObjectStorage>, String), Status> {
        let settings = self.settings()?;
        let requested = bucket
            .filter(|b| !b.is_empty())
            .unwrap_or(settings.bucket.as_str());
        if requested == settings.bucket {
            let engine = self.state.oss.clone().ok_or_else(oss_missing)?;
            return Ok((engine, settings.bucket.clone()));
        }
        {
            let cache = self.extra_engines.lock().expect("oss engine cache");
            if let Some(engine) = cache.get(requested) {
                return Ok((engine.clone(), requested.to_string()));
            }
        }
        let engine = rushwind_oss_s3::S3Storage::new(rushwind_oss::StorageConfig {
            endpoint: settings.endpoint.clone(),
            region: settings.region.clone(),
            access_key: settings.access_key.clone(),
            secret_key: settings.secret_key.clone(),
            token: None,
            use_ssl: settings.use_ssl,
            force_path_style: settings.force_path_style,
            bucket: requested.to_string(),
        })
        .map_err(|e| Status::internal(format!("oss engine for bucket {requested}: {e:?}")))?;
        // First-use bucket setup (best-effort, idempotent) — the same
        // guarantee the default bucket gets at startup. The concrete
        // S3Storage carries the setup helper; the trait does not.
        if let Err(e) = engine.create_bucket().await {
            tracing::warn!("oss bucket {requested} setup: {e:?} (continuing)");
        }
        let arc: Arc<dyn rushwind_oss::ObjectStorage> = Arc::new(engine);
        self.extra_engines
            .lock()
            .expect("oss engine cache")
            .insert(requested.to_string(), arc.clone());
        Ok((arc, requested.to_string()))
    }
}

#[async_trait::async_trait]
impl storagev1::oss_service_server::OssService for OssService {
    /// The client-side upload grant: a presigned PUT (the engine
    /// carries the signing authority; POST-form grants stay the
    /// PUT spelling this round), plus the read URL of where the
    /// object will land.
    async fn get_upload_presigned_url(
        &self,
        request: Request<storagev1::GetUploadPresignedUrlRequest>,
    ) -> Result<Response<storagev1::GetUploadPresignedUrlResponse>, Status> {
        use rushwind_oss::PresignMethod;
        let req = request.into_inner();
        let directory = req
            .file_directory
            .clone()
            .unwrap_or_default()
            .trim_matches('/')
            .to_string();
        let object_name = unique_object_name(req.file_name.as_deref().unwrap_or(""));
        let key = if directory.is_empty() {
            object_name.clone()
        } else {
            format!("{directory}/{object_name}")
        };
        let expires = std::time::Duration::from_secs(
            req.expire_seconds.unwrap_or(600).clamp(1, 60 * 60 * 24) as u64,
        );
        let (engine, bucket) = self.engine_for(req.bucket_name.as_deref()).await?;
        let upload_url = engine
            .presign(&key, PresignMethod::Put, expires)
            .map_err(|e| Status::internal(format!("presign: {e:?}")))?;
        Ok(Response::new(storagev1::GetUploadPresignedUrlResponse {
            upload_url,
            download_url: format!("/api/v1/storage/oss/download/{}", percent_encode(&key)),
            bucket_name: Some(bucket),
            object_name: key,
            form_data: Default::default(),
        }))
    }

    /// The read grant: resolve by file ledger id or by explicit object
    /// name, then presign a GET.
    async fn get_download_url(
        &self,
        request: Request<storagev1::GetDownloadInfoRequest>,
    ) -> Result<Response<storagev1::GetDownloadInfoResponse>, Status> {
        use rushwind_oss::PresignMethod;
        let req = request.into_inner();
        let mut req_bucket: Option<String> = None;
        let key = match req.selector {
            Some(storagev1::get_download_info_request::Selector::FileId(id)) => {
                use sea_orm::EntityTrait;
                let row = store::entities::files::Entity::find_by_id(id)
                    .one(&self.state.db)
                    .await
                    .map_err(crate::state::db_status)?
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
            Some(storagev1::get_download_info_request::Selector::StorageObject(object)) => {
                let directory = object
                    .file_directory
                    .unwrap_or_default()
                    .trim_matches('/')
                    .to_string();
                let name = object
                    .object_name
                    .unwrap_or_default()
                    .trim_start_matches('/')
                    .to_string();
                if name.is_empty() {
                    return Err(bad("storage_object.objectName required"));
                }
                if directory.is_empty() {
                    name
                } else {
                    format!("{directory}/{name}")
                }
            }
            Some(storagev1::get_download_info_request::Selector::DownloadUrl(_)) => {
                // The external-URL selector has no object key of ours to
                // presign — the direct face reads it as an object name.
                return Err(bad("download_url selector carries no object key"));
            }
            None => return Err(bad("selector required")),
        };
        let expires = std::time::Duration::from_secs(
            req.presign_expire_seconds
                .unwrap_or(600)
                .clamp(1, 60 * 60 * 24) as u64,
        );
        let (engine, bucket) = self.engine_for(req_bucket.as_deref()).await?;
        let url = engine
            .presign(&key, PresignMethod::Get, expires)
            .map_err(|e| Status::internal(format!("presign: {e:?}")))?;
        Ok(Response::new(storagev1::GetDownloadInfoResponse {
            storage_path: key,
            content: Some(storagev1::get_download_info_response::Content::DownloadUrl(
                url,
            )),
            ..Default::default()
        }))
    }

    /// The bucket listing under a folder prefix.
    async fn list_oss_file(
        &self,
        request: Request<storagev1::ListOssFileRequest>,
    ) -> Result<Response<storagev1::ListOssFileResponse>, Status> {
        let req = request.into_inner();
        let prefix = req.folder.unwrap_or_default().trim_matches('/').to_string();
        let prefix = if prefix.is_empty() {
            String::new()
        } else {
            format!("{prefix}/")
        };
        let (engine, _bucket) = self.engine_for(req.bucket_name.as_deref()).await?;
        let objects = engine
            .list(&prefix, "", 1000)
            .await
            .map_err(|e| Status::internal(format!("list: {e:?}")))?;
        let files = objects.into_iter().map(|o| o.key).collect();
        Ok(Response::new(storagev1::ListOssFileResponse { files }))
    }

    async fn delete_oss_file(
        &self,
        request: Request<storagev1::DeleteOssFileRequest>,
    ) -> Result<Response<storagev1::DeleteOssFileResponse>, Status> {
        let req = request.into_inner();
        let object_name = req
            .object_name
            .filter(|s| !s.is_empty())
            .ok_or_else(|| bad("objectName required"))?;
        let (engine, _bucket) = self.engine_for(req.bucket_name.as_deref()).await?;
        engine
            .delete(&object_name)
            .await
            .map_err(|e| Status::internal(format!("delete: {e:?}")))?;
        Ok(Response::new(storagev1::DeleteOssFileResponse {}))
    }
}

/// The %2F-safe key spelling a URL path segment carries.
fn percent_encode(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    for b in key.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
