mod cleanup;
mod ops;
mod path;

use std::sync::Arc;
use crate::runtime_settings::RuntimeSettings;

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

    pub async fn apply_runtime_settings(&self, settings: RuntimeSettings) -> Result<(), crate::error::AppError> {
        self.validate_runtime_settings(&settings).await?;

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

    async fn local_health_status(&self, settings: &RuntimeSettings) -> crate::models::ComponentStatus {
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
