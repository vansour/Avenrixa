use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use aws_sdk_s3::primitives::ByteStream;
use tokio::fs;
use uuid::Uuid;

use super::StorageManager;
use super::path::build_s3_object_key;
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
                self.resolve_s3_client(&settings).await?;
                Ok(())
            }
        }
    }

    pub async fn test_s3_settings(&self, settings: &RuntimeSettings) -> Result<(), AppError> {
        let client = Self::build_s3_client(settings)?;
        let probe_file_key = format!(".vansour-s3-probe-{}", Uuid::new_v4().simple());
        let object_key = build_s3_object_key(client.prefix.as_deref(), &probe_file_key);

        client
            .client
            .put_object()
            .bucket(client.bucket.clone())
            .key(object_key.clone())
            .body(ByteStream::from_static(b"vansour-s3-probe"))
            .send()
            .await
            .map_err(|e| AppError::ValidationError(format!("S3 上传测试失败: {}", e)))?;

        client
            .client
            .get_object()
            .bucket(client.bucket.clone())
            .key(object_key.clone())
            .send()
            .await
            .map_err(|e| AppError::ValidationError(format!("S3 读取测试失败: {}", e)))?;

        client
            .client
            .delete_object()
            .bucket(client.bucket)
            .key(object_key)
            .send()
            .await
            .map_err(|e| AppError::ValidationError(format!("S3 清理测试文件失败: {}", e)))?;

        Ok(())
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

    fn sample_s3_runtime_settings() -> RuntimeSettings {
        RuntimeSettings {
            site_name: "Vansour Image".to_string(),
            storage_backend: StorageBackend::S3,
            local_storage_path: "/tmp/vansour-image".to_string(),
            mail_enabled: false,
            mail_smtp_host: String::new(),
            mail_smtp_port: 587,
            mail_smtp_user: None,
            mail_smtp_password: None,
            mail_from_email: String::new(),
            mail_from_name: String::new(),
            mail_link_base_url: String::new(),
            s3_endpoint: Some("https://example.r2.cloudflarestorage.com".to_string()),
            s3_region: Some("auto".to_string()),
            s3_bucket: Some("images".to_string()),
            s3_prefix: Some("uploads".to_string()),
            s3_access_key: Some("access".to_string()),
            s3_secret_key: Some("secret".to_string()),
            s3_force_path_style: false,
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

    #[tokio::test]
    async fn check_health_allows_valid_s3_configuration_without_remote_probe() {
        let manager = StorageManager::new(sample_s3_runtime_settings());

        manager
            .check_health()
            .await
            .expect("s3 health should only validate client construction");
    }
}
