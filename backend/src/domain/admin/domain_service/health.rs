use chrono::Utc;

use super::AdminDomainService;
use crate::cache::CacheCommands;
use crate::db::DatabasePool;
use crate::error::AppError;
use crate::models::{ComponentStatus, HealthMetrics, HealthState, HealthStatus};
use crate::runtime_settings::StorageBackend;

fn build_version_label(
    app_version: Option<&str>,
    fallback_version: &str,
    _revision: Option<&str>,
) -> String {
    app_version
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback_version)
        .to_string()
}

fn app_version_label() -> String {
    build_version_label(
        option_env!("APP_VERSION"),
        env!("CARGO_PKG_VERSION"),
        option_env!("APP_REVISION"),
    )
}

fn describe_storage_backend(settings: &crate::runtime_settings::RuntimeSettings) -> String {
    match settings.storage_backend {
        StorageBackend::Local => {
            format!("本地存储 · {}", settings.local_storage_path.trim())
        }
        StorageBackend::S3 => {
            let bucket = settings.s3_bucket.as_deref().unwrap_or("未配置桶");
            let endpoint = settings.s3_endpoint.as_deref().unwrap_or("未配置 endpoint");
            let provider = infer_s3_provider(
                settings.s3_endpoint.as_deref(),
                settings.s3_region.as_deref(),
            );
            format!("{provider} · {bucket} · {endpoint}")
        }
    }
}

fn infer_s3_provider(endpoint: Option<&str>, region: Option<&str>) -> &'static str {
    let endpoint = endpoint
        .map(str::trim)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let region = region
        .map(str::trim)
        .unwrap_or_default()
        .to_ascii_lowercase();

    if endpoint.contains(".r2.cloudflarestorage.com") || region == "auto" {
        "Cloudflare R2"
    } else if endpoint.contains("amazonaws.com") {
        "AWS S3"
    } else if endpoint.contains("minio") || endpoint.contains("9000") {
        "MinIO / S3"
    } else {
        "对象存储"
    }
}

impl AdminDomainService {
    #[tracing::instrument(skip(self))]
    pub async fn health_check(&self, uptime_seconds: u64) -> Result<HealthStatus, AppError> {
        let timestamp = Utc::now();
        let mut overall_status = HealthState::Healthy;

        let db_status = match self.database_ping().await {
            Ok(_) => ComponentStatus::healthy(),
            Err(e) => {
                overall_status = HealthState::Unhealthy;
                ComponentStatus::unhealthy(e.to_string())
            }
        };

        let cache_status = if let Some(manager) = self.cache.as_ref() {
            let mut cache = manager.clone();
            match cache.ping::<()>().await {
                Ok(_) => ComponentStatus::healthy(),
                Err(e) => {
                    if overall_status == HealthState::Healthy {
                        overall_status = HealthState::Degraded;
                    }
                    ComponentStatus::degraded(format!("外部缓存不可用，已降级为无缓存模式: {}", e))
                }
            }
        } else {
            if self.cache_status.status == HealthState::Degraded
                && overall_status == HealthState::Healthy
            {
                overall_status = HealthState::Degraded;
            }
            self.cache_status.clone()
        };

        let storage_settings = self.storage_manager.active_settings();
        let storage_probe_status = self.storage_manager.health_component_status().await;
        let storage_status = ComponentStatus {
            status: storage_probe_status.status,
            message: Some(storage_status_message(
                &storage_settings,
                storage_probe_status.message.as_deref(),
            )),
        };
        match storage_status.status {
            HealthState::Healthy | HealthState::Disabled => {}
            HealthState::Degraded => {
                if overall_status == HealthState::Healthy {
                    overall_status = HealthState::Degraded;
                }
            }
            _ => {
                overall_status = HealthState::Unhealthy;
            }
        }

        let metrics = self.collect_health_metrics().await.ok();

        Ok(HealthStatus {
            status: overall_status,
            timestamp,
            database: db_status,
            cache: cache_status,
            storage: storage_status,
            version: Some(app_version_label()),
            uptime_seconds: Some(uptime_seconds),
            metrics,
        })
    }

    async fn collect_health_metrics(&self) -> Result<HealthMetrics, AppError> {
        let images_count: i64 = match &self.database {
            DatabasePool::Postgres(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM images WHERE status = 'active'")
                    .fetch_one(pool)
                    .await
                    .unwrap_or(0)
            }
            DatabasePool::MySql(pool) => sqlx::query_scalar(
                "SELECT CAST(COUNT(*) AS SIGNED) FROM images WHERE status = 'active'",
            )
            .fetch_one(pool)
            .await
            .unwrap_or(0),
            DatabasePool::Sqlite(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM images WHERE status = 'active'")
                    .fetch_one(pool)
                    .await
                    .unwrap_or(0)
            }
        };

        let users_count: i64 = match &self.database {
            DatabasePool::Postgres(pool) => sqlx::query_scalar("SELECT COUNT(*) FROM users")
                .fetch_one(pool)
                .await
                .unwrap_or(0),
            DatabasePool::MySql(pool) => {
                sqlx::query_scalar("SELECT CAST(COUNT(*) AS SIGNED) FROM users")
                    .fetch_one(pool)
                    .await
                    .unwrap_or(0)
            }
            DatabasePool::Sqlite(pool) => sqlx::query_scalar("SELECT COUNT(*) FROM users")
                .fetch_one(pool)
                .await
                .unwrap_or(0),
        };

        let storage_used_mb = self.storage_used_mb().await;

        Ok(HealthMetrics {
            images_count,
            users_count,
            storage_used_mb,
        })
    }

    async fn database_ping(&self) -> Result<(), sqlx::Error> {
        match &self.database {
            DatabasePool::Postgres(pool) => {
                sqlx::query_scalar::<_, i32>("SELECT 1")
                    .fetch_one(pool)
                    .await?;
            }
            DatabasePool::MySql(pool) => {
                sqlx::query_scalar::<_, i32>("SELECT 1")
                    .fetch_one(pool)
                    .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query_scalar::<_, i32>("SELECT 1")
                    .fetch_one(pool)
                    .await?;
            }
        }
        Ok(())
    }

    async fn storage_used_mb(&self) -> Option<f64> {
        let total_size = match &self.database {
            DatabasePool::Postgres(pool) => sqlx::query_scalar::<_, Option<i64>>(
                "SELECT CAST(SUM(size) AS BIGINT) FROM images WHERE status = 'active'",
            )
            .fetch_one(pool)
            .await
            .unwrap_or(None),
            DatabasePool::MySql(pool) => sqlx::query_scalar::<_, Option<i64>>(
                "SELECT CAST(SUM(size) AS SIGNED) FROM images WHERE status = 'active'",
            )
            .fetch_one(pool)
            .await
            .unwrap_or(None),
            DatabasePool::Sqlite(pool) => sqlx::query_scalar::<_, Option<i64>>(
                "SELECT SUM(size) FROM images WHERE status = 'active'",
            )
            .fetch_one(pool)
            .await
            .unwrap_or(None),
        };
        total_size.map(|size| size as f64 / 1024.0 / 1024.0)
    }
}

fn storage_status_message(
    settings: &crate::runtime_settings::RuntimeSettings,
    probe_message: Option<&str>,
) -> String {
    match probe_message {
        Some(probe_message) if !probe_message.trim().is_empty() => {
            format!("{} | {}", describe_storage_backend(settings), probe_message)
        }
        _ => describe_storage_backend(settings),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_version_label, describe_storage_backend, infer_s3_provider, storage_status_message,
    };
    use crate::runtime_settings::{RuntimeSettings, StorageBackend};

    const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

    #[test]
    fn build_version_label_falls_back_to_package_version() {
        assert_eq!(
            build_version_label(None, PACKAGE_VERSION, None),
            PACKAGE_VERSION
        );
    }

    #[test]
    fn build_version_label_prefers_explicit_app_version() {
        assert_eq!(
            build_version_label(Some("0.1.1"), PACKAGE_VERSION, None),
            "0.1.1"
        );
    }

    #[test]
    fn build_version_label_ignores_revision_when_present() {
        assert_eq!(
            build_version_label(Some("0.1.2-rc.1"), "ignored", Some("abc123def456")),
            "0.1.2-rc.1"
        );
    }

    #[test]
    fn build_version_label_ignores_dev_revision_placeholders() {
        assert_eq!(
            build_version_label(Some(PACKAGE_VERSION), "ignored", Some("dev")),
            PACKAGE_VERSION
        );
    }

    fn sample_runtime_settings() -> RuntimeSettings {
        RuntimeSettings {
            site_name: "Avenrixa".to_string(),
            storage_backend: StorageBackend::S3,
            local_storage_path: "/data/images".to_string(),
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
            s3_bucket: Some("bucket".to_string()),
            s3_prefix: Some("images".to_string()),
            s3_access_key: Some("access".to_string()),
            s3_secret_key: Some("secret".to_string()),
            s3_force_path_style: false,
        }
    }

    #[test]
    fn storage_status_message_appends_probe_layers() {
        let message = storage_status_message(
            &sample_runtime_settings(),
            Some("配置=正常 | 远端探测=正常 | 读写=未执行写探测"),
        );

        assert!(message.contains("Cloudflare R2"));
        assert!(message.contains("配置=正常"));
    }

    #[test]
    fn infer_s3_provider_detects_r2() {
        assert_eq!(
            infer_s3_provider(Some("https://demo.r2.cloudflarestorage.com"), Some("auto")),
            "Cloudflare R2"
        );
    }

    #[test]
    fn describe_storage_backend_formats_s3_summary() {
        let settings = RuntimeSettings {
            site_name: "Avenrixa".to_string(),
            storage_backend: StorageBackend::S3,
            local_storage_path: "/data/images".to_string(),
            mail_enabled: false,
            mail_smtp_host: String::new(),
            mail_smtp_port: 587,
            mail_smtp_user: None,
            mail_smtp_password: None,
            mail_from_email: String::new(),
            mail_from_name: String::new(),
            mail_link_base_url: String::new(),
            s3_endpoint: Some("https://demo.r2.cloudflarestorage.com".to_string()),
            s3_region: Some("auto".to_string()),
            s3_bucket: Some("images".to_string()),
            s3_prefix: None,
            s3_access_key: None,
            s3_secret_key: None,
            s3_force_path_style: false,
        };

        assert_eq!(
            describe_storage_backend(&settings),
            "Cloudflare R2 · images · https://demo.r2.cloudflarestorage.com"
        );
    }
}
