mod apply;

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{Mutex, RwLock};

use self::apply::{PersistAndApplyInput, persist_and_apply};
use crate::config::Config;
use crate::db::DatabasePool;
use crate::error::AppError;
use crate::models::UpdateAdminSettingsConfigRequest;
use crate::storage_backend::StorageManager;

use super::model::{RuntimeSettings, admin_setting_policy};
use super::store::load_from_db;
use super::validation::{validate_and_merge, validate_raw_setting_update};

const SETTINGS_CACHE_TTL: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct RuntimeSettingsService {
    pool: DatabasePool,
    defaults: RuntimeSettings,
    cache: Arc<RwLock<Option<(Instant, RuntimeSettings)>>>,
    update_lock: Arc<Mutex<()>>,
}

impl RuntimeSettingsService {
    pub fn new(pool: DatabasePool, config: &Config) -> Self {
        Self {
            pool,
            defaults: RuntimeSettings::from_defaults(config),
            cache: Arc::new(RwLock::new(None)),
            update_lock: Arc::new(Mutex::new(())),
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

        self.load_runtime_settings_uncached().await
    }

    async fn load_runtime_settings_uncached(&self) -> Result<RuntimeSettings, AppError> {
        let fetched = load_from_db(&self.pool, &self.defaults).await?;
        let mut guard = self.cache.write().await;
        *guard = Some((Instant::now(), fetched.clone()));
        Ok(fetched)
    }

    pub async fn update_admin_settings_config(
        &self,
        req: UpdateAdminSettingsConfigRequest,
        storage_manager: &StorageManager,
    ) -> Result<RuntimeSettings, AppError> {
        let _guard = self.update_lock.lock().await;
        let previous = self.load_runtime_settings_uncached().await?;
        self.ensure_expected_settings_version(req.expected_settings_version.as_deref(), &previous)?;
        let validated = validate_and_merge(previous.clone(), req)?;
        persist_and_apply(
            self,
            &PersistAndApplyInput {
                previous: &previous,
                validated: &validated,
                storage_manager,
            },
        )
        .await
    }

    pub async fn update_raw_setting(
        &self,
        key: &str,
        value: &str,
        storage_manager: &StorageManager,
    ) -> Result<RuntimeSettings, AppError> {
        let _guard = self.update_lock.lock().await;
        if !admin_setting_policy(key).editable {
            return Err(AppError::ValidationError(
                "该设置项受保护，不能通过高级设置直接修改".to_string(),
            ));
        }

        let previous = self.load_runtime_settings_uncached().await?;
        let validated = validate_raw_setting_update(previous.clone(), key, value)?;
        persist_and_apply(
            self,
            &PersistAndApplyInput {
                previous: &previous,
                validated: &validated,
                storage_manager,
            },
        )
        .await
    }

    fn ensure_expected_settings_version(
        &self,
        expected: Option<&str>,
        current: &RuntimeSettings,
    ) -> Result<(), AppError> {
        let Some(expected) = expected.map(str::trim).filter(|value| !value.is_empty()) else {
            return Ok(());
        };
        let current_version = current.settings_version();
        if expected == current_version {
            return Ok(());
        }

        Err(AppError::Conflict(
            "设置已被其他管理员更新，请刷新后重新确认变更".to_string(),
        ))
    }
}
