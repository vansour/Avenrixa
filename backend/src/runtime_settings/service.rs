use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use crate::config::Config;
use crate::db::DatabasePool;
use crate::error::AppError;
use crate::models::UpdateAdminSettingsConfigRequest;

use super::model::{RuntimeSettings, admin_setting_policy};
use super::store::{load_from_db, persist_settings};
use super::validation::{validate_and_merge, validate_raw_setting_update};

const SETTINGS_CACHE_TTL: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct RuntimeSettingsService {
    pool: DatabasePool,
    defaults: RuntimeSettings,
    cache: Arc<RwLock<Option<(Instant, RuntimeSettings)>>>,
}

impl RuntimeSettingsService {
    pub fn new(pool: DatabasePool, config: &Config) -> Self {
        Self {
            pool,
            defaults: RuntimeSettings::from_defaults(config),
            cache: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn invalidate_cache(&self) {
        let mut guard = self.cache.write().await;
        *guard = None;
    }

    pub async fn get_runtime_settings(&self) -> Result<RuntimeSettings, AppError> {
        if let Some((loaded_at, settings)) = self.cache.read().await.as_ref()
            && loaded_at.elapsed() < SETTINGS_CACHE_TTL
        {
            return Ok(settings.clone());
        }

        let fetched = load_from_db(&self.pool, &self.defaults).await?;
        let mut guard = self.cache.write().await;
        *guard = Some((Instant::now(), fetched.clone()));
        Ok(fetched)
    }

    pub async fn update_admin_settings_config(
        &self,
        req: UpdateAdminSettingsConfigRequest,
    ) -> Result<RuntimeSettings, AppError> {
        let current = self.get_runtime_settings().await?;
        let validated = validate_and_merge(current, req)?;
        persist_settings(&self.pool, &validated).await?;
        self.invalidate_cache().await;
        self.get_runtime_settings().await
    }

    pub async fn update_raw_setting(
        &self,
        key: &str,
        value: &str,
    ) -> Result<RuntimeSettings, AppError> {
        if !admin_setting_policy(key).editable {
            return Err(AppError::ValidationError(
                "该设置项受保护，不能通过高级设置直接修改".to_string(),
            ));
        }

        let current = self.get_runtime_settings().await?;
        let validated = validate_raw_setting_update(current, key, value)?;
        persist_settings(&self.pool, &validated).await?;
        self.invalidate_cache().await;
        self.get_runtime_settings().await
    }
}
