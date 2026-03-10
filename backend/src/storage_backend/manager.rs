use std::sync::Arc;

use tokio::fs;

use super::StorageManager;
use crate::error::AppError;
use crate::runtime_settings::{RuntimeSettings, StorageBackend};

impl StorageManager {
    pub fn new(active_settings: RuntimeSettings) -> Self {
        Self {
            active_settings,
            s3_client_cache: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    pub fn active_settings(&self) -> &RuntimeSettings {
        &self.active_settings
    }

    pub fn restart_required(&self, settings: &RuntimeSettings) -> bool {
        self.active_settings.storage_settings() != settings.storage_settings()
    }

    pub async fn check_health(&self) -> Result<(), AppError> {
        let settings = self.active_settings();
        match settings.storage_backend {
            StorageBackend::Local => {
                fs::metadata(&settings.local_storage_path).await?;
                Ok(())
            }
            StorageBackend::S3 => {
                let cached = self.resolve_s3_client(settings).await?;
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
