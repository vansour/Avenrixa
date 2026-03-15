use chrono::Utc;

use super::AdminDomainService;
use crate::cache::CacheCommands;
use crate::db::DatabasePool;
use crate::error::AppError;
use crate::models::{ComponentStatus, HealthMetrics, HealthState, HealthStatus};

fn build_version_label(
    app_version: Option<&str>,
    fallback_version: &str,
    revision: Option<&str>,
) -> String {
    let version = app_version
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback_version);

    let revision = revision
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "unknown" && *value != "dev");

    match revision {
        Some(revision) => format!("{version} ({revision})"),
        None => version.to_string(),
    }
}

fn app_version_label() -> String {
    build_version_label(
        option_env!("APP_VERSION"),
        env!("CARGO_PKG_VERSION"),
        option_env!("APP_REVISION"),
    )
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

        let storage_status = match self.storage_manager.check_health().await {
            Ok(_) => ComponentStatus::healthy(),
            Err(e) => {
                overall_status = HealthState::Unhealthy;
                ComponentStatus::unhealthy(e.to_string())
            }
        };

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

#[cfg(test)]
mod tests {
    use super::build_version_label;

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
            build_version_label(Some("0.1.0"), PACKAGE_VERSION, None),
            "0.1.0"
        );
    }

    #[test]
    fn build_version_label_appends_revision_when_present() {
        assert_eq!(
            build_version_label(Some(PACKAGE_VERSION), "ignored", Some("abc123def456")),
            format!("{PACKAGE_VERSION} (abc123def456)")
        );
    }

    #[test]
    fn build_version_label_ignores_dev_revision_placeholders() {
        assert_eq!(
            build_version_label(Some(PACKAGE_VERSION), "ignored", Some("dev")),
            PACKAGE_VERSION
        );
    }
}
