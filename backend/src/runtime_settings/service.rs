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

    fn request_from_settings(
        settings: &RuntimeSettings,
        expected_settings_version: Option<String>,
    ) -> UpdateAdminSettingsConfigRequest {
        UpdateAdminSettingsConfigRequest {
            expected_settings_version,
            site_name: settings.site_name.clone(),
            storage_backend: StorageBackendKind::Local,
            local_storage_path: settings.local_storage_path.clone(),
            mail_enabled: settings.mail_enabled,
            mail_smtp_host: settings.mail_smtp_host.clone(),
            mail_smtp_port: Some(settings.mail_smtp_port),
            mail_smtp_user: settings.mail_smtp_user.clone(),
            mail_smtp_password: settings.mail_smtp_password.clone(),
            mail_from_email: settings.mail_from_email.clone(),
            mail_from_name: settings.mail_from_name.clone(),
            mail_link_base_url: settings.mail_link_base_url.clone(),
            s3_endpoint: settings.s3_endpoint.clone(),
            s3_region: settings.s3_region.clone(),
            s3_bucket: settings.s3_bucket.clone(),
            s3_prefix: settings.s3_prefix.clone(),
            s3_access_key: settings.s3_access_key.clone(),
            s3_secret_key: settings.s3_secret_key.clone(),
            s3_force_path_style: Some(settings.s3_force_path_style),
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
                    local_storage_path: requested_path.to_string_lossy().into_owned(),
                    ..request_from_settings(&original, Some(original.settings_version()))
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

    #[tokio::test]
    async fn update_admin_settings_config_rejects_stale_settings_version() {
        let context = setup_service().await;
        let original = context
            .service
            .get_runtime_settings()
            .await
            .expect("initial settings should load");
        let original_version = original.settings_version();

        let first_request = UpdateAdminSettingsConfigRequest {
            site_name: "First Update".to_string(),
            ..request_from_settings(&original, Some(original_version.clone()))
        };
        let updated = context
            .service
            .update_admin_settings_config(first_request, &context.storage_manager)
            .await
            .expect("first update should succeed");
        assert_eq!(updated.site_name, "First Update");

        let stale_request = UpdateAdminSettingsConfigRequest {
            local_storage_path: context
                ._temp_dir
                .path()
                .join("stale-write")
                .to_string_lossy()
                .into_owned(),
            ..request_from_settings(&original, Some(original_version))
        };
        let error = context
            .service
            .update_admin_settings_config(stale_request, &context.storage_manager)
            .await
            .expect_err("stale request should be rejected");

        assert!(matches!(error, AppError::Conflict(message) if message.contains("刷新后重新确认")));

        let current = context
            .service
            .get_runtime_settings()
            .await
            .expect("current settings should load");
        assert_eq!(current.site_name, "First Update");
        assert_eq!(current.local_storage_path, original.local_storage_path);
    }

    #[tokio::test]
    async fn update_admin_settings_config_checks_expected_version_against_fresh_db_state() {
        let context = setup_service().await;
        let original = context
            .service
            .get_runtime_settings()
            .await
            .expect("initial settings should load");
        let original_version = original.settings_version();

        let mut externally_persisted = original.clone();
        externally_persisted.site_name = "External Update".to_string();
        super::super::store::persist_settings(
            &DatabasePool::Sqlite(context.pool.clone()),
            &externally_persisted,
        )
        .await
        .expect("external write should succeed");

        let stale_request = UpdateAdminSettingsConfigRequest {
            site_name: "Local Draft".to_string(),
            ..request_from_settings(&original, Some(original_version))
        };
        let error = context
            .service
            .update_admin_settings_config(stale_request, &context.storage_manager)
            .await
            .expect_err("stale request should be rejected after external write");

        assert!(matches!(error, AppError::Conflict(message) if message.contains("刷新后重新确认")));

        let current = context
            .service
            .get_runtime_settings()
            .await
            .expect("current settings should load");
        assert_eq!(current.site_name, "External Update");
    }

    #[tokio::test]
    async fn update_raw_setting_merges_against_fresh_db_state() {
        let context = setup_service().await;
        let original = context
            .service
            .get_runtime_settings()
            .await
            .expect("initial settings should load");

        let mut externally_persisted = original.clone();
        externally_persisted.site_name = "External Update".to_string();
        super::super::store::persist_settings(
            &DatabasePool::Sqlite(context.pool.clone()),
            &externally_persisted,
        )
        .await
        .expect("external write should succeed");

        let updated = context
            .service
            .update_raw_setting(
                super::super::model::SETTING_MAIL_FROM_NAME,
                "Updated Sender",
                &context.storage_manager,
            )
            .await
            .expect("raw update should succeed");

        assert_eq!(updated.site_name, "External Update");
        assert_eq!(updated.mail_from_name, "Updated Sender");
    }
}
