use crate::config::Config;
use crate::error::AppError;
use crate::models::{AdminSettingsConfig, UpdateAdminSettingsConfigRequest};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub const SETTING_SITE_NAME: &str = "site_name";
pub const SETTING_STORAGE_BACKEND: &str = "storage_backend";
pub const SETTING_LOCAL_STORAGE_PATH: &str = "local_storage_path";
pub const SETTING_S3_ENDPOINT: &str = "s3_endpoint";
pub const SETTING_S3_REGION: &str = "s3_region";
pub const SETTING_S3_BUCKET: &str = "s3_bucket";
pub const SETTING_S3_PREFIX: &str = "s3_prefix";
pub const SETTING_S3_ACCESS_KEY: &str = "s3_access_key";
pub const SETTING_S3_SECRET_KEY: &str = "s3_secret_key";
pub const SETTING_S3_FORCE_PATH_STYLE: &str = "s3_force_path_style";

const SETTINGS_CACHE_TTL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageBackend {
    Local,
    S3,
}

impl StorageBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::S3 => "s3",
        }
    }

    pub fn parse(value: &str) -> Self {
        if value.eq_ignore_ascii_case("s3") {
            Self::S3
        } else {
            Self::Local
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeSettings {
    pub site_name: String,
    pub storage_backend: StorageBackend,
    pub local_storage_path: String,
    pub s3_endpoint: Option<String>,
    pub s3_region: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_prefix: Option<String>,
    pub s3_access_key: Option<String>,
    pub s3_secret_key: Option<String>,
    pub s3_force_path_style: bool,
}

impl RuntimeSettings {
    pub fn from_defaults(config: &Config) -> Self {
        let env_site_name = std::env::var("SITE_NAME")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let env_backend = std::env::var("STORAGE_BACKEND")
            .ok()
            .map(|v| StorageBackend::parse(v.trim()));
        let env_s3_endpoint = std::env::var("S3_ENDPOINT")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let env_s3_region = std::env::var("S3_REGION")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let env_s3_bucket = std::env::var("S3_BUCKET")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let env_s3_prefix = std::env::var("S3_PREFIX")
            .ok()
            .map(|v| normalize_s3_prefix(v.trim()));
        let env_s3_access_key = std::env::var("S3_ACCESS_KEY")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let env_s3_secret_key = std::env::var("S3_SECRET_KEY")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let env_s3_force_path_style = std::env::var("S3_FORCE_PATH_STYLE")
            .ok()
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(true);

        Self {
            site_name: env_site_name.unwrap_or_else(|| "Vansour Image".to_string()),
            storage_backend: env_backend.unwrap_or(StorageBackend::Local),
            local_storage_path: config.storage.path.clone(),
            s3_endpoint: env_s3_endpoint,
            s3_region: env_s3_region,
            s3_bucket: env_s3_bucket,
            s3_prefix: env_s3_prefix,
            s3_access_key: env_s3_access_key,
            s3_secret_key: env_s3_secret_key,
            s3_force_path_style: env_s3_force_path_style,
        }
    }

    pub fn to_admin_config(&self) -> AdminSettingsConfig {
        AdminSettingsConfig {
            site_name: self.site_name.clone(),
            storage_backend: self.storage_backend.as_str().to_string(),
            local_storage_path: self.local_storage_path.clone(),
            s3_endpoint: self.s3_endpoint.clone(),
            s3_region: self.s3_region.clone(),
            s3_bucket: self.s3_bucket.clone(),
            s3_prefix: self.s3_prefix.clone(),
            s3_access_key: self.s3_access_key.clone(),
            s3_secret_key_set: self
                .s3_secret_key
                .as_ref()
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false),
            s3_force_path_style: self.s3_force_path_style,
            restart_required: true,
        }
    }
}

#[derive(Clone)]
pub struct RuntimeSettingsService {
    pool: PgPool,
    defaults: RuntimeSettings,
    cache: Arc<RwLock<Option<(Instant, RuntimeSettings)>>>,
}

impl RuntimeSettingsService {
    pub fn new(pool: PgPool, config: &Config) -> Self {
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

        let fetched = self.load_from_db().await?;
        let mut guard = self.cache.write().await;
        *guard = Some((Instant::now(), fetched.clone()));
        Ok(fetched)
    }

    pub async fn get_admin_settings_config(&self) -> Result<AdminSettingsConfig, AppError> {
        Ok(self.get_runtime_settings().await?.to_admin_config())
    }

    pub async fn update_admin_settings_config(
        &self,
        req: UpdateAdminSettingsConfigRequest,
    ) -> Result<AdminSettingsConfig, AppError> {
        let current = self.get_runtime_settings().await?;
        let validated = Self::validate_and_merge(current, req)?;

        let mut tx = self.pool.begin().await?;
        upsert_setting(&mut tx, SETTING_SITE_NAME, &validated.site_name).await?;
        upsert_setting(
            &mut tx,
            SETTING_STORAGE_BACKEND,
            validated.storage_backend.as_str(),
        )
        .await?;
        upsert_setting(
            &mut tx,
            SETTING_LOCAL_STORAGE_PATH,
            &validated.local_storage_path,
        )
        .await?;
        upsert_setting_opt(
            &mut tx,
            SETTING_S3_ENDPOINT,
            validated.s3_endpoint.as_deref(),
        )
        .await?;
        upsert_setting_opt(&mut tx, SETTING_S3_REGION, validated.s3_region.as_deref()).await?;
        upsert_setting_opt(&mut tx, SETTING_S3_BUCKET, validated.s3_bucket.as_deref()).await?;
        upsert_setting_opt(&mut tx, SETTING_S3_PREFIX, validated.s3_prefix.as_deref()).await?;
        upsert_setting_opt(
            &mut tx,
            SETTING_S3_ACCESS_KEY,
            validated.s3_access_key.as_deref(),
        )
        .await?;
        upsert_setting_opt(
            &mut tx,
            SETTING_S3_SECRET_KEY,
            validated.s3_secret_key.as_deref(),
        )
        .await?;
        upsert_setting(
            &mut tx,
            SETTING_S3_FORCE_PATH_STYLE,
            if validated.s3_force_path_style {
                "true"
            } else {
                "false"
            },
        )
        .await?;
        tx.commit().await?;

        self.invalidate_cache().await;
        self.get_admin_settings_config().await
    }

    pub async fn get_site_name(&self) -> Result<String, AppError> {
        Ok(self.get_runtime_settings().await?.site_name)
    }

    async fn load_from_db(&self) -> Result<RuntimeSettings, AppError> {
        let rows = sqlx::query_as::<_, (String, String)>("SELECT key, value FROM settings")
            .fetch_all(&self.pool)
            .await?;
        let mut kv = HashMap::new();
        for (k, v) in rows {
            kv.insert(k, v);
        }

        let mut settings = self.defaults.clone();
        if let Some(site_name) = kv
            .get(SETTING_SITE_NAME)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
        {
            settings.site_name = site_name;
        }
        if let Some(storage_backend) = kv.get(SETTING_STORAGE_BACKEND) {
            settings.storage_backend = StorageBackend::parse(storage_backend.trim());
        }
        if let Some(local_path) = kv
            .get(SETTING_LOCAL_STORAGE_PATH)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
        {
            settings.local_storage_path = local_path;
        }

        settings.s3_endpoint = kv
            .get(SETTING_S3_ENDPOINT)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        settings.s3_region = kv
            .get(SETTING_S3_REGION)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        settings.s3_bucket = kv
            .get(SETTING_S3_BUCKET)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        settings.s3_prefix = kv
            .get(SETTING_S3_PREFIX)
            .map(|v| normalize_s3_prefix(v.trim()));
        settings.s3_access_key = kv
            .get(SETTING_S3_ACCESS_KEY)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        settings.s3_secret_key = kv
            .get(SETTING_S3_SECRET_KEY)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        settings.s3_force_path_style = kv
            .get(SETTING_S3_FORCE_PATH_STYLE)
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(true);

        Ok(settings)
    }

    fn validate_and_merge(
        mut current: RuntimeSettings,
        req: UpdateAdminSettingsConfigRequest,
    ) -> Result<RuntimeSettings, AppError> {
        let site_name = req.site_name.trim();
        if site_name.is_empty() || site_name.chars().count() > 120 {
            return Err(AppError::InvalidPagination);
        }
        current.site_name = site_name.to_string();

        current.storage_backend = StorageBackend::parse(req.storage_backend.trim());

        let local_path = req.local_storage_path.trim();
        if local_path.is_empty() {
            return Err(AppError::InvalidPagination);
        }
        current.local_storage_path = local_path.to_string();

        current.s3_endpoint = req
            .s3_endpoint
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        current.s3_region = req
            .s3_region
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        current.s3_bucket = req
            .s3_bucket
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        current.s3_prefix = req
            .s3_prefix
            .as_ref()
            .map(|v| normalize_s3_prefix(v.trim()));
        current.s3_access_key = req
            .s3_access_key
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        if let Some(secret) = req.s3_secret_key {
            let normalized = secret.trim().to_string();
            if !normalized.is_empty() {
                current.s3_secret_key = Some(normalized);
            }
        }
        current.s3_force_path_style = req.s3_force_path_style.unwrap_or(true);

        if current.storage_backend == StorageBackend::S3 {
            let endpoint_ok = current
                .s3_endpoint
                .as_ref()
                .map(|v| !v.is_empty())
                .unwrap_or(false);
            let region_ok = current
                .s3_region
                .as_ref()
                .map(|v| !v.is_empty())
                .unwrap_or(false);
            let bucket_ok = current
                .s3_bucket
                .as_ref()
                .map(|v| !v.is_empty())
                .unwrap_or(false);
            let access_ok = current
                .s3_access_key
                .as_ref()
                .map(|v| !v.is_empty())
                .unwrap_or(false);
            let secret_ok = current
                .s3_secret_key
                .as_ref()
                .map(|v| !v.is_empty())
                .unwrap_or(false);
            if !(endpoint_ok && region_ok && bucket_ok && access_ok && secret_ok) {
                return Err(AppError::InvalidPagination);
            }
        }

        Ok(current)
    }
}

fn normalize_s3_prefix(raw: &str) -> String {
    raw.trim_matches('/').to_string()
}

async fn upsert_setting(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    key: &str,
    value: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at)
         VALUES ($1, $2, NOW())
         ON CONFLICT (key)
         DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()",
    )
    .bind(key)
    .bind(value)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn upsert_setting_opt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    key: &str,
    value: Option<&str>,
) -> Result<(), AppError> {
    match value {
        Some(v) if !v.trim().is_empty() => upsert_setting(tx, key, v).await?,
        _ => {
            sqlx::query("DELETE FROM settings WHERE key = $1")
                .bind(key)
                .execute(&mut **tx)
                .await?;
        }
    }
    Ok(())
}
