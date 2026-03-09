use std::sync::Arc;

use tokio::fs;

use super::StorageManager;
use crate::error::AppError;
use crate::runtime_settings::{RuntimeSettings, RuntimeSettingsService, StorageBackend};

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
                    .bucket(cached.bucket.clone())
                    .send()
                    .await
                    .map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("S3 health check failed: {}", e))
                    })?;
                Ok(())
            }
        }
    }

    pub fn cache_hint(&self, file_key: &str) -> String {
        format!("storage://{}", file_key)
    }
}
