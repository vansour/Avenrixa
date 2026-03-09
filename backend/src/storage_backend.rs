use crate::error::AppError;
use crate::runtime_settings::{RuntimeSettings, RuntimeSettingsService, StorageBackend};
use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::config::{Builder as S3ConfigBuilder, Region};
use aws_sdk_s3::primitives::ByteStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;

#[derive(Clone)]
pub struct StorageManager {
    runtime_settings: Arc<RuntimeSettingsService>,
    s3_client_cache: Arc<tokio::sync::RwLock<Option<CachedS3Client>>>,
}

#[derive(Clone)]
struct CachedS3Client {
    signature: String,
    bucket: String,
    prefix: Option<String>,
    client: S3Client,
}

impl StorageManager {
    pub fn new(runtime_settings: Arc<RuntimeSettingsService>) -> Self {
        Self {
            runtime_settings,
            s3_client_cache: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    pub async fn runtime_settings(&self) -> Result<RuntimeSettings, AppError> {
        self.runtime_settings.get_runtime_settings().await
    }

    pub async fn ensure_local_storage_dir(&self) -> Result<(), AppError> {
        let settings = self.runtime_settings().await?;
        let local_path = settings.local_storage_path;
        fs::create_dir_all(local_path).await?;
        Ok(())
    }

    pub async fn check_health(&self) -> Result<(), AppError> {
        let settings = self.runtime_settings().await?;
        match settings.storage_backend {
            StorageBackend::Local => {
                fs::metadata(settings.local_storage_path).await?;
                Ok(())
            }
            StorageBackend::S3 => {
                let cached = self.resolve_s3_client(&settings).await?;
                cached
                    .client
                    .head_bucket()
                    .bucket(cached.bucket)
                    .send()
                    .await
                    .map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("S3 health check failed: {}", e))
                    })?;
                Ok(())
            }
        }
    }

    pub async fn exists(&self, file_key: &str) -> Result<bool, AppError> {
        validate_file_key(file_key)?;
        let settings = self.runtime_settings().await?;
        match settings.storage_backend {
            StorageBackend::Local => {
                let path = join_local_path(&settings.local_storage_path, file_key)?;
                Ok(fs::try_exists(path).await.unwrap_or(false))
            }
            StorageBackend::S3 => {
                let cached = self.resolve_s3_client(&settings).await?;
                let object_key = build_s3_object_key(cached.prefix.as_deref(), file_key);
                match cached
                    .client
                    .head_object()
                    .bucket(cached.bucket)
                    .key(object_key)
                    .send()
                    .await
                {
                    Ok(_) => Ok(true),
                    Err(err) => {
                        if let Some(service_err) = err.as_service_error()
                            && service_err.is_not_found()
                        {
                            return Ok(false);
                        }
                        Err(AppError::Internal(anyhow::anyhow!(
                            "Failed to check S3 object existence: {}",
                            err
                        )))
                    }
                }
            }
        }
    }

    pub async fn read(&self, file_key: &str) -> Result<Vec<u8>, AppError> {
        validate_file_key(file_key)?;
        let settings = self.runtime_settings().await?;
        match settings.storage_backend {
            StorageBackend::Local => {
                let path = join_local_path(&settings.local_storage_path, file_key)?;
                Ok(fs::read(path).await?)
            }
            StorageBackend::S3 => {
                let cached = self.resolve_s3_client(&settings).await?;
                let object_key = build_s3_object_key(cached.prefix.as_deref(), file_key);
                let resp = cached
                    .client
                    .get_object()
                    .bucket(cached.bucket)
                    .key(object_key)
                    .send()
                    .await
                    .map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("Failed to read S3 object: {}", e))
                    })?;
                let bytes = resp
                    .body
                    .collect()
                    .await
                    .map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("Failed to collect S3 bytes: {}", e))
                    })?
                    .into_bytes();
                Ok(bytes.to_vec())
            }
        }
    }

    pub async fn write(&self, file_key: &str, data: &[u8]) -> Result<(), AppError> {
        validate_file_key(file_key)?;
        let settings = self.runtime_settings().await?;
        match settings.storage_backend {
            StorageBackend::Local => {
                let path = join_local_path(&settings.local_storage_path, file_key)?;
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).await?;
                }
                fs::write(path, data).await?;
                Ok(())
            }
            StorageBackend::S3 => {
                let cached = self.resolve_s3_client(&settings).await?;
                let object_key = build_s3_object_key(cached.prefix.as_deref(), file_key);
                cached
                    .client
                    .put_object()
                    .bucket(cached.bucket)
                    .key(object_key)
                    .body(ByteStream::from(data.to_vec()))
                    .send()
                    .await
                    .map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("Failed to write S3 object: {}", e))
                    })?;
                Ok(())
            }
        }
    }

    pub async fn delete(&self, file_key: &str) -> Result<(), AppError> {
        validate_file_key(file_key)?;
        let settings = self.runtime_settings().await?;
        match settings.storage_backend {
            StorageBackend::Local => {
                let path = join_local_path(&settings.local_storage_path, file_key)?;
                match fs::remove_file(path).await {
                    Ok(_) => Ok(()),
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    Err(err) => Err(AppError::IoError(err)),
                }
            }
            StorageBackend::S3 => {
                let cached = self.resolve_s3_client(&settings).await?;
                let object_key = build_s3_object_key(cached.prefix.as_deref(), file_key);
                cached
                    .client
                    .delete_object()
                    .bucket(cached.bucket)
                    .key(object_key)
                    .send()
                    .await
                    .map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("Failed to delete S3 object: {}", e))
                    })?;
                Ok(())
            }
        }
    }

    pub async fn copy(&self, src_key: &str, dst_key: &str) -> Result<(), AppError> {
        validate_file_key(src_key)?;
        validate_file_key(dst_key)?;
        let settings = self.runtime_settings().await?;
        match settings.storage_backend {
            StorageBackend::Local => {
                let src = join_local_path(&settings.local_storage_path, src_key)?;
                let dst = join_local_path(&settings.local_storage_path, dst_key)?;
                if let Some(parent) = dst.parent() {
                    fs::create_dir_all(parent).await?;
                }
                fs::copy(src, dst).await?;
                Ok(())
            }
            StorageBackend::S3 => {
                let cached = self.resolve_s3_client(&settings).await?;
                let src_obj_key = build_s3_object_key(cached.prefix.as_deref(), src_key);
                let dst_obj_key = build_s3_object_key(cached.prefix.as_deref(), dst_key);
                let copy_source = format!("{}/{}", cached.bucket, src_obj_key);
                cached
                    .client
                    .copy_object()
                    .bucket(cached.bucket)
                    .key(dst_obj_key)
                    .copy_source(copy_source)
                    .send()
                    .await
                    .map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("Failed to copy S3 object: {}", e))
                    })?;
                Ok(())
            }
        }
    }

    pub fn cache_hint(&self, file_key: &str) -> String {
        format!("storage://{}", file_key)
    }

    async fn resolve_s3_client(
        &self,
        settings: &RuntimeSettings,
    ) -> Result<CachedS3Client, AppError> {
        let endpoint = settings
            .s3_endpoint
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .ok_or(AppError::InvalidPagination)?;
        let region = settings
            .s3_region
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .ok_or(AppError::InvalidPagination)?;
        let bucket = settings
            .s3_bucket
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .ok_or(AppError::InvalidPagination)?;
        let access_key = settings
            .s3_access_key
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .ok_or(AppError::InvalidPagination)?;
        let secret_key = settings
            .s3_secret_key
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .ok_or(AppError::InvalidPagination)?;
        let prefix = settings
            .s3_prefix
            .as_ref()
            .map(|v| v.trim_matches('/').to_string())
            .filter(|v| !v.is_empty());
        let signature = format!(
            "{}|{}|{}|{}|{}|{}",
            endpoint, region, bucket, access_key, secret_key, settings.s3_force_path_style
        );

        if let Some(cached) = self.s3_client_cache.read().await.as_ref()
            && cached.signature == signature
        {
            return Ok(cached.clone());
        }

        let creds = Credentials::new(access_key, secret_key, None, None, "runtime-settings");
        let mut builder = S3ConfigBuilder::new()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(region))
            .credentials_provider(creds)
            .endpoint_url(endpoint);
        if settings.s3_force_path_style {
            builder = builder.force_path_style(true);
        }
        let client = S3Client::from_conf(builder.build());

        let next = CachedS3Client {
            signature,
            bucket,
            prefix,
            client,
        };
        let mut guard = self.s3_client_cache.write().await;
        *guard = Some(next.clone());
        Ok(next)
    }
}

fn validate_file_key(file_key: &str) -> Result<(), AppError> {
    let key = file_key.trim();
    if key.is_empty()
        || key.contains('/')
        || key.contains('\\')
        || key.contains("..")
        || key.len() > 255
    {
        return Err(AppError::InvalidPagination);
    }
    Ok(())
}

fn join_local_path(base: &str, file_key: &str) -> Result<PathBuf, AppError> {
    validate_file_key(file_key)?;
    let path = Path::new(base).join(file_key);
    Ok(path)
}

fn build_s3_object_key(prefix: Option<&str>, file_key: &str) -> String {
    match prefix {
        Some(prefix) if !prefix.is_empty() => format!("{}/{}", prefix.trim_matches('/'), file_key),
        _ => file_key.to_string(),
    }
}
