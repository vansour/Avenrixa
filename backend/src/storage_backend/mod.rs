mod cleanup;
mod ops;
mod path;

use crate::runtime_settings::RuntimeSettings;
use std::sync::Arc;

pub use cleanup::{enqueue_storage_cleanup_jobs, process_pending_storage_cleanup_jobs};

#[derive(Clone)]
pub struct StorageManager {
    pub(super) active_settings: Arc<std::sync::RwLock<RuntimeSettings>>,
}

impl StorageManager {
    pub fn new(active_settings: RuntimeSettings) -> Self {
        Self {
            active_settings: Arc::new(std::sync::RwLock::new(active_settings)),
        }
    }

    pub fn active_settings(&self) -> RuntimeSettings {
        self.active_settings
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn restart_required(&self, settings: &RuntimeSettings) -> bool {
        self.active_settings().storage_backend != settings.storage_backend
    }

    pub async fn validate_runtime_settings(
        &self,
        settings: &RuntimeSettings,
    ) -> Result<(), crate::error::AppError> {
        if settings.storage_backend != crate::runtime_settings::StorageBackend::Local {
            return Err(crate::error::AppError::ValidationError(
                "仅支持本地存储".to_string(),
            ));
        }

        if settings.local_storage_path.trim().is_empty() {
            return Err(crate::error::AppError::ValidationError(
                "本地存储路径不能为空".to_string(),
            ));
        }

        Ok(())
    }

    pub async fn apply_runtime_settings(
        &self,
        settings: RuntimeSettings,
    ) -> Result<(), crate::error::AppError> {
        self.validate_runtime_settings(&settings).await?;
        tokio::fs::create_dir_all(&settings.local_storage_path).await?;

        {
            let mut guard = self
                .active_settings
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *guard = settings;
        }

        Ok(())
    }

    pub async fn health_component_status(&self) -> crate::models::ComponentStatus {
        let settings = self.active_settings();
        self.local_health_status(&settings).await
    }

    async fn local_health_status(
        &self,
        settings: &RuntimeSettings,
    ) -> crate::models::ComponentStatus {
        let path = std::path::Path::new(&settings.local_storage_path);
        if path.exists() {
            crate::models::ComponentStatus {
                status: crate::models::HealthState::Healthy,
                message: Some("配置=正常 | 路径访问=正常 | 读写=本地文件系统".to_string()),
            }
        } else {
            crate::models::ComponentStatus::unhealthy(
                "配置=正常 | 路径访问=失败 | 读写=不可用".to_string(),
            )
        }
    }

    pub fn cache_hint(&self, _file_key: &str) -> String {
        "storage://local".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::StorageManager;
    use crate::runtime_settings::{RuntimeSettings, StorageBackend};

    fn sample_runtime_settings(local_storage_path: String) -> RuntimeSettings {
        RuntimeSettings {
            site_name: "Avenrixa".to_string(),
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
        }
    }

    #[tokio::test]
    async fn apply_runtime_settings_creates_local_storage_directory() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let storage_path = temp_dir.path().join("images");
        let manager = StorageManager::new(sample_runtime_settings(
            temp_dir
                .path()
                .join("bootstrap")
                .to_string_lossy()
                .into_owned(),
        ));
        let settings = sample_runtime_settings(storage_path.to_string_lossy().into_owned());

        manager
            .apply_runtime_settings(settings)
            .await
            .expect("runtime settings should be applied");

        assert!(
            tokio::fs::try_exists(&storage_path)
                .await
                .expect("storage path existence check should succeed")
        );
    }
}
