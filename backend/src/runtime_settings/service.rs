use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use crate::config::Config;
use crate::db::DatabasePool;
use crate::error::AppError;
use crate::models::UpdateAdminSettingsConfigRequest;
use crate::storage_backend::StorageManager;
use tracing::error;

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
        storage_manager: &StorageManager,
    ) -> Result<RuntimeSettings, AppError> {
        let current = self.get_runtime_settings().await?;
        let validated = validate_and_merge(current, req)?;
        self.persist_and_apply(validated, storage_manager).await
    }

    pub async fn update_raw_setting(
        &self,
        key: &str,
        value: &str,
        storage_manager: &StorageManager,
    ) -> Result<RuntimeSettings, AppError> {
        if !admin_setting_policy(key).editable {
            return Err(AppError::ValidationError(
                "该设置项受保护，不能通过高级设置直接修改".to_string(),
            ));
        }

        let current = self.get_runtime_settings().await?;
        let validated = validate_raw_setting_update(current, key, value)?;
        self.persist_and_apply(validated, storage_manager).await
    }

    async fn persist_and_apply(
        &self,
        validated: RuntimeSettings,
        storage_manager: &StorageManager,
    ) -> Result<RuntimeSettings, AppError> {
        let previous = self.get_runtime_settings().await?;
        storage_manager
            .validate_runtime_settings(&validated)
            .await?;
        persist_settings(&self.pool, &validated).await?;

        if let Err(apply_error) = storage_manager
            .apply_runtime_settings(validated.clone())
            .await
        {
            let rollback_result = self
                .rollback_after_apply_failure(&previous, storage_manager)
                .await;
            return match rollback_result {
                Ok(()) => Err(apply_error),
                Err(rollback_error) => {
                    error!(
                        "runtime settings apply failed and rollback failed: apply={}, rollback={}",
                        apply_error, rollback_error
                    );
                    Err(AppError::Internal(anyhow::anyhow!(
                        "运行时设置应用失败，且回滚失败: apply={}, rollback={}",
                        apply_error,
                        rollback_error
                    )))
                }
            };
        }

        self.invalidate_cache().await;
        self.get_runtime_settings().await
    }

    async fn rollback_after_apply_failure(
        &self,
        previous: &RuntimeSettings,
        storage_manager: &StorageManager,
    ) -> Result<(), AppError> {
        persist_settings(&self.pool, previous).await?;
        self.invalidate_cache().await;
        storage_manager
            .apply_runtime_settings(previous.clone())
            .await?;
        self.invalidate_cache().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DatabaseKind};
    use crate::db::{DatabasePool, get_setting_value, run_migrations};
    use crate::models::StorageBackendKind;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use tempfile::TempDir;

    struct TestContext {
        _temp_dir: TempDir,
        pool: sqlx::SqlitePool,
        service: RuntimeSettingsService,
        storage_manager: StorageManager,
    }

    async fn setup_service() -> TestContext {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let database_path = temp_dir.path().join("runtime-settings.db");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&database_path)
                    .create_if_missing(true)
                    .foreign_keys(true),
            )
            .await
            .expect("sqlite pool should be created");
        let database = DatabasePool::Sqlite(pool.clone());
        run_migrations(&database)
            .await
            .expect("sqlite migrations should succeed");

        let mut config = Config::default();
        config.database.kind = DatabaseKind::Sqlite;
        config.database.url = database_path.to_string_lossy().into_owned();
        config.storage.path = temp_dir
            .path()
            .join("storage")
            .to_string_lossy()
            .into_owned();
        let service = RuntimeSettingsService::new(database, &config);
        let storage_manager = StorageManager::new(service.get_runtime_settings().await.unwrap());

        TestContext {
            _temp_dir: temp_dir,
            pool,
            service,
            storage_manager,
        }
    }

    #[tokio::test]
    async fn update_admin_settings_config_rolls_back_persisted_settings_when_apply_fails() {
        let context = setup_service().await;
        let original = context
            .service
            .get_runtime_settings()
            .await
            .expect("initial settings should load");
        let requested_path = context._temp_dir.path().join("applied-later");

        context.storage_manager.fail_next_apply_for_tests();
        let error = context
            .service
            .update_admin_settings_config(
                UpdateAdminSettingsConfigRequest {
                    site_name: "Rollback Test".to_string(),
                    storage_backend: StorageBackendKind::Local,
                    local_storage_path: requested_path.to_string_lossy().into_owned(),
                    mail_enabled: false,
                    mail_smtp_host: String::new(),
                    mail_smtp_port: Some(587),
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
                    s3_force_path_style: Some(true),
                },
                &context.storage_manager,
            )
            .await
            .expect_err("apply failure should bubble up");

        assert!(matches!(error, AppError::Internal(_)));
        assert_eq!(
            context.storage_manager.active_settings().local_storage_path,
            original.local_storage_path
        );

        let persisted_path = get_setting_value(
            &DatabasePool::Sqlite(context.pool.clone()),
            super::super::model::SETTING_LOCAL_STORAGE_PATH,
        )
        .await
        .expect("path should load")
        .expect("path should be persisted after rollback");
        assert_eq!(persisted_path, original.local_storage_path);

        let current = context
            .service
            .get_runtime_settings()
            .await
            .expect("rolled back settings should load");
        assert_eq!(current.local_storage_path, original.local_storage_path);
        assert_eq!(current.site_name, original.site_name);
    }
}
