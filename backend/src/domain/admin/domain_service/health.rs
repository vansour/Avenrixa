use chrono::Utc;
use redis::AsyncCommands;

use super::AdminDomainService;
use crate::error::AppError;
use crate::models::{ComponentStatus, HealthMetrics, HealthStatus};

impl AdminDomainService {
    #[tracing::instrument(skip(self))]
    pub async fn health_check(&self, uptime_seconds: u64) -> Result<HealthStatus, AppError> {
        use sqlx::Executor;

        let timestamp = Utc::now();
        let mut overall_status = "healthy".to_string();

        let db_status = match self.pool.acquire().await {
            Ok(mut conn) => match conn.execute(sqlx::query("SELECT 1")).await {
                Ok(_) => ComponentStatus::healthy(),
                Err(e) => {
                    overall_status = "unhealthy".to_string();
                    ComponentStatus::unhealthy(e.to_string())
                }
            },
            Err(e) => {
                overall_status = "unhealthy".to_string();
                ComponentStatus::unhealthy(e.to_string())
            }
        };

        let redis_status = if let Some(manager) = self.redis.as_ref() {
            let mut redis = manager.clone();
            match redis.ping::<()>().await {
                Ok(_) => ComponentStatus::healthy(),
                Err(e) => {
                    overall_status = "unhealthy".to_string();
                    ComponentStatus::unhealthy(e.to_string())
                }
            }
        } else {
            ComponentStatus::unhealthy("Redis not configured")
        };

        let storage_status = match self.storage_manager.check_health().await {
            Ok(_) => ComponentStatus::healthy(),
            Err(e) => {
                overall_status = "unhealthy".to_string();
                ComponentStatus::unhealthy(e.to_string())
            }
        };

        let metrics = self.collect_health_metrics().await.ok();

        Ok(HealthStatus {
            status: overall_status,
            timestamp,
            database: db_status,
            redis: redis_status,
            storage: storage_status,
            version: option_env!("APP_VERSION").map(|s| s.to_string()),
            uptime_seconds: Some(uptime_seconds),
            metrics,
        })
    }

    async fn collect_health_metrics(&self) -> Result<HealthMetrics, AppError> {
        let images_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM images WHERE deleted_at IS NULL")
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);

        let users_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        let storage_used_mb: Option<f64> = {
            let total_size: Option<i64> =
                sqlx::query_scalar("SELECT CAST(SUM(size) AS BIGINT) FROM images")
                    .fetch_one(&self.pool)
                    .await
                    .unwrap_or(None);
            total_size.map(|size| size as f64 / 1024.0 / 1024.0)
        };

        Ok(HealthMetrics {
            images_count,
            users_count,
            storage_used_mb,
        })
    }
}
