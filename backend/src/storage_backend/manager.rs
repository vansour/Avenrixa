use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use tokio::fs;

use super::StorageManager;
use crate::error::AppError;
use crate::runtime_settings::{RuntimeSettings, StorageBackend};

impl StorageManager {
    pub fn new(active_settings: RuntimeSettings) -> Self {
        Self {
            active_settings: Arc::new(std::sync::RwLock::new(active_settings)),
            s3_client_cache: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    pub fn active_settings(&self) -> RuntimeSettings {
        self.active_settings
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn restart_required(&self, settings: &RuntimeSettings) -> bool {
        self.active_settings().storage_settings() != settings.storage_settings()
    }

    pub async fn apply_runtime_settings(&self, settings: RuntimeSettings) -> Result<(), AppError> {
        if settings.storage_backend == StorageBackend::Local {
            fs::create_dir_all(&settings.local_storage_path).await?;
            let images_perms = PermissionsExt::from_mode(0o755);
            let _ = fs::set_permissions(&settings.local_storage_path, images_perms).await;
        }

        {
            let mut guard = self
                .active_settings
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *guard = settings;
        }

        *self.s3_client_cache.write().await = None;
        Ok(())
    }

    pub async fn check_health(&self) -> Result<(), AppError> {
        let settings = self.active_settings();
        match settings.storage_backend {
            StorageBackend::Local => {
                fs::metadata(&settings.local_storage_path).await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_runtime_settings(local_storage_path: String) -> RuntimeSettings {
        RuntimeSettings {
            site_name: "Vansour Image".to_string(),
            storage_backend: StorageBackend::Local,
            local_storage_path,
            mail_enabled: false,
            mail_smtp_host: String::new(),
            mail_smtp_port: 587,
            mail_smtp_user: None,
            mail_smtp_password: None,
            mail_from_email: String::new(),
            mail_from_name: String::new(),
            mail_link_base_url: String::new(),
            s3_endpoint: None,
            s3_region: None,
            s3_bucket: None,
            s3_prefix: None,
            s3_access_key: None,
            s3_secret_key: None,
            s3_force_path_style: true,
        }
    }

    #[tokio::test]
    async fn apply_runtime_settings_updates_local_path_and_creates_directory() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let initial_path = temp_dir.path().join("initial");
        let next_path = temp_dir.path().join("mounted").join("images");
        let manager = StorageManager::new(sample_runtime_settings(
            initial_path.to_string_lossy().into(),
        ));

        manager
            .apply_runtime_settings(sample_runtime_settings(next_path.to_string_lossy().into()))
            .await
            .expect("runtime settings should apply");

        assert_eq!(
            manager.active_settings().local_storage_path,
            next_path.to_string_lossy().to_string()
        );
        assert!(
            tokio::fs::try_exists(&next_path)
                .await
                .expect("existence check should succeed")
        );
    }
}
