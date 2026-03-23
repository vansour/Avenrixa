use chrono::Utc;

use super::AdminDomainService;
use crate::db::DatabasePool;
use crate::error::AppError;
use crate::models::{
    ComponentStatus, HealthMetrics, HealthState, HealthStatus, RuntimeObservabilitySnapshot,
};
use crate::runtime_settings::StorageBackend;

struct HealthMetricsCollection {
    metrics: HealthMetrics,
    warnings: Vec<String>,
}

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

fn cache_disabled_status() -> ComponentStatus {
    ComponentStatus::disabled("未配置外部缓存，当前以无缓存模式运行")
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
            let cache = manager.clone();
            match cache.ping().await {
                Ok(_) => ComponentStatus::healthy(),
                Err(e) => {
                    if overall_status == HealthState::Healthy {
                        overall_status = HealthState::Degraded;
                    }
                    ComponentStatus::degraded(format!("外部缓存不可用，已降级为无缓存模式: {}", e))
                }
            }
        } else {
            cache_disabled_status()
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
        };

        let metrics = self.collect_health_metrics().await;
        let observability_snapshot = match self.collect_runtime_observability().await {
            Ok(snapshot) => snapshot,
            Err(error) => {
                let mut warnings = metrics.warnings.clone();
                warnings.push(format!("运行态指标查询失败: {}", error));
                let observability = observability_status(None, &warnings);
                if overall_status == HealthState::Healthy {
                    overall_status = observability.status;
                }
                return Ok(HealthStatus {
                    status: overall_status,
                    timestamp,
                    database: db_status,
                    cache: cache_status,
                    storage: storage_status,
                    observability,
                    version: Some(app_version_label()),
                    uptime_seconds: Some(uptime_seconds),
                    metrics: Some(metrics.metrics),
                });
            }
        };

        let observability = observability_status(Some(&observability_snapshot), &metrics.warnings);
        if overall_status == HealthState::Healthy && observability.status == HealthState::Degraded {
            overall_status = HealthState::Degraded;
        }

        Ok(HealthStatus {
            status: overall_status,
            timestamp,
            database: db_status,
            cache: cache_status,
            storage: storage_status,
            observability,
            version: Some(app_version_label()),
            uptime_seconds: Some(uptime_seconds),
            metrics: Some(metrics.metrics),
        })
    }

    async fn collect_health_metrics(&self) -> HealthMetricsCollection {
        let mut warnings = Vec::new();

        let images_count = match &self.database {
            DatabasePool::Postgres(pool) => {
                match sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM images WHERE status = 'active'",
                )
                .fetch_one(pool)
                .await
                {
                    Ok(value) => Some(value),
                    Err(error) => {
                        warnings.push(format!("活动图片统计查询失败: {}", error));
                        None
                    }
                }
            }
        };

        let users_count = match &self.database {
            DatabasePool::Postgres(pool) => {
                match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
                    .fetch_one(pool)
                    .await
                {
                    Ok(value) => Some(value),
                    Err(error) => {
                        warnings.push(format!("用户统计查询失败: {}", error));
                        None
                    }
                }
            }
        };

        let storage_used_mb = match self.storage_used_mb().await {
            Ok(value) => value,
            Err(error) => {
                warnings.push(format!("存储占用统计查询失败: {}", error));
                None
            }
        };

        HealthMetricsCollection {
            metrics: HealthMetrics {
                images_count,
                users_count,
                storage_used_mb,
            },
            warnings,
        }
    }

    async fn database_ping(&self) -> Result<(), sqlx::Error> {
        match &self.database {
            DatabasePool::Postgres(pool) => {
                sqlx::query_scalar::<_, i32>("SELECT 1")
                    .fetch_one(pool)
                    .await?;
            }
        }
        Ok(())
    }

    async fn storage_used_mb(&self) -> Result<Option<f64>, sqlx::Error> {
        match &self.database {
            DatabasePool::Postgres(pool) => {
                let size = sqlx::query_scalar::<_, Option<i64>>(
                    "SELECT CAST(SUM(size) AS BIGINT) FROM images WHERE status = 'active'",
                )
                .fetch_one(pool)
                .await?;
                Ok(size.map(|value| value as f64 / 1024.0 / 1024.0))
            }
        }
    }
}

fn observability_status(
    snapshot: Option<&RuntimeObservabilitySnapshot>,
    warnings: &[String],
) -> ComponentStatus {
    let mut degraded_reasons = warnings.to_vec();
    let mut summary = Vec::new();

    if let Some(snapshot) = snapshot {
        let failing_tasks = snapshot
            .background_tasks
            .iter()
            .filter(|task| task.consecutive_failures > 0)
            .map(|task| {
                format!(
                    "{} 连续失败 {} 次",
                    task.task_name, task.consecutive_failures
                )
            })
            .collect::<Vec<_>>();
        degraded_reasons.extend(failing_tasks);

        if snapshot.backlog.storage_cleanup_retrying > 0 {
            degraded_reasons.push(format!(
                "存储清理重试队列 {} 项",
                snapshot.backlog.storage_cleanup_retrying
            ));
        }

        summary.push(format!(
            "存储清理积压 {} 项",
            snapshot.backlog.storage_cleanup_pending
        ));
        summary.push(format!("后台任务 {} 个", snapshot.background_tasks.len()));
        summary.push(format!(
            "审计失败 {} 次",
            snapshot.audit_writes.total_failures
        ));
    }

    if !degraded_reasons.is_empty() {
        return ComponentStatus::degraded(degraded_reasons.join("；"));
    }

    ComponentStatus {
        status: HealthState::Healthy,
        message: Some(if summary.is_empty() {
            "关键运行态指标已启用".to_string()
        } else {
            summary.join("；")
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_version_label, cache_disabled_status, describe_storage_backend,
        storage_status_message,
    };
    use crate::models::HealthState;
    use crate::runtime_settings::{RuntimeSettings, StorageBackend};

    const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

    #[test]
    fn build_version_label_calls_package_version_when_present() {
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
            build_version_label(Some("0.1.2-rc.3"), "ignored", Some("abc123def456")),
            "0.1.2-rc.3"
        );
    }

    #[test]
    fn build_version_label_ignores_dev_revision_placeholders() {
        assert_eq!(
            build_version_label(Some(PACKAGE_VERSION), "dev", None),
            PACKAGE_VERSION
        );
    }

    fn sample_runtime_settings() -> RuntimeSettings {
        RuntimeSettings {
            site_name: "Avenrixa".to_string(),
            storage_backend: StorageBackend::Local,
            local_storage_path: "/data/images".to_string(),
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

    #[test]
    fn storage_status_message_appends_probe_layers() {
        let message = storage_status_message(
            &sample_runtime_settings(),
            Some("配置=正常 | 路径访问=正常 | 读写=本地文件系统"),
        );
        assert!(message.contains("本地存储"));
        assert!(message.contains("配置=正常"));
        assert!(message.contains("路径访问=正常"));
    }

    #[test]
    fn describe_storage_backend_formats_local_summary() {
        let settings = sample_runtime_settings();
        assert_eq!(
            describe_storage_backend(&settings),
            "本地存储 · /data/images"
        );
    }

    #[test]
    fn cache_disabled_status_reports_disabled_component() {
        let status = cache_disabled_status();

        assert_eq!(status.status, HealthState::Disabled);
        assert_eq!(
            status.message.as_deref(),
            Some("未配置外部缓存，当前以无缓存模式运行")
        );
    }
}
