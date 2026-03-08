//! 管理领域服务
//!
//! 封装管理相关的业务逻辑

use chrono::Utc;
use sqlx::PgPool;
use tracing::{error, info};
use uuid::Uuid;

use crate::audit::log_audit;
use crate::config::Config;
use crate::error::AppError;
use crate::models::{
    AuditLog, AuditLogResponse, ComponentStatus, HealthMetrics, HealthStatus, Setting, SystemStats,
    User,
};

use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// 管理领域服务
pub struct AdminDomainService {
    pool: PgPool,
    redis: Option<ConnectionManager>,
    config: Config,
}

impl AdminDomainService {
    pub fn new(pool: PgPool, redis: Option<ConnectionManager>, config: Config) -> Self {
        Self {
            pool,
            redis,
            config,
        }
    }

    /// 健康检查
    #[tracing::instrument(skip(self))]
    pub async fn health_check(&self, uptime_seconds: u64) -> Result<HealthStatus, AppError> {
        use sqlx::Executor;

        let timestamp = Utc::now();
        let mut overall_status = "healthy".to_string();

        // 检查数据库
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

        // 检查 Redis
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

        // 检查存储
        let storage_path = &self.config.storage.path;
        let storage_status = match tokio::fs::metadata(storage_path).await {
            Ok(_) => ComponentStatus::healthy(),
            Err(e) => {
                overall_status = "unhealthy".to_string();
                ComponentStatus::unhealthy(e.to_string())
            }
        };

        // 收集系统指标
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

    /// 数据库备份
    pub async fn backup_database(
        &self,
        admin_user_id: Uuid,
        admin_username: &str,
    ) -> Result<crate::models::BackupResponse, AppError> {
        let filename = format!("backup_{}.sql", Uuid::new_v4());
        let backup_dir = "/data/backup";
        let backup_path = format!("{}/{}", backup_dir, filename);

        // 创建备份目录
        tokio::fs::create_dir_all(backup_dir).await?;

        // 获取数据库连接信息
        let database_url = &self.config.database.url;

        // 使用 pg_dump 执行真正的数据库备份
        let child = tokio::process::Command::new("pg_dump")
            .arg("--dbname")
            .arg(database_url)
            .arg("--format=plain")
            .arg("--no-owner")
            .arg("--no-acl")
            .stdout(std::process::Stdio::piped())
            .spawn();

        let mut child = match child {
            Ok(child) => child,
            Err(e) => {
                error!("Failed to spawn pg_dump process: {}", e);
                return Err(AppError::Internal(anyhow::anyhow!(
                    "Failed to execute pg_dump: {}",
                    e
                )));
            }
        };

        // 读取pg_dump 输出并写入文件
        let mut stdout = child.stdout.take().expect("Failed to capture stdout");
        let mut buffer = Vec::new();
        let _ = stdout.read_to_end(&mut buffer).await;

        // 创建备份文件
        let mut file = tokio::fs::File::create(&backup_path).await?;
        file.write_all(&buffer).await?;

        // 等待进程完成
        let status = child.wait().await;
        match status {
            Ok(status) => {
                if !status.success() {
                    error!("pg_dump failed with status: {}", status);
                    let _ = tokio::fs::remove_file(&backup_path).await;
                    return Err(AppError::Internal(anyhow::anyhow!(
                        "pg_dump failed with exit code: {}",
                        status
                    )));
                }
            }
            Err(e) => {
                error!("Failed to wait for pg_dump: {}", e);
                let _ = tokio::fs::remove_file(&backup_path).await;
                return Err(AppError::Internal(anyhow::anyhow!(
                    "pg_dump wait error: {}",
                    e
                )));
            }
        }

        info!(
            "Database backup created: {} by {}",
            filename, admin_username
        );

        // 记录审计日志
        let pool = self.pool.clone();
        let admin_id = admin_user_id;
        let filename_clone = filename.clone();
        tokio::spawn(async move {
            let _ = log_audit(
                &pool,
                Some(admin_id),
                "backup_created",
                "backup",
                None,
                None,
                Some(serde_json::json!({"filename": filename_clone})),
            )
            .await;
        });

        Ok(crate::models::BackupResponse {
            filename,
            created_at: Utc::now(),
        })
    }

    /// 收集健康指标
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
            let total_size: Option<i64> = sqlx::query_scalar("SELECT SUM(size) FROM images")
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

    /// 清理已删除的文件
    pub async fn cleanup_deleted_files(
        &self,
        admin_user_id: Uuid,
        admin_username: &str,
    ) -> Result<Vec<String>, AppError> {
        let now = Utc::now();
        let retention_days = self.config.cleanup.deleted_images_retention_days;
        let days_ago = now - chrono::Duration::days(retention_days);

        let result = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, filename FROM images WHERE deleted_at < $1",
        )
        .bind(days_ago)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to query cleanup images: {}", e);
            let pool = self.pool.clone();
            let error_msg = e.to_string();
            let admin_id = admin_user_id;
            tokio::spawn(async move {
                let _ = log_audit(
                    &pool,
                    Some(admin_id),
                    "cleanup_failed",
                    "cleanup",
                    None,
                    None,
                    Some(serde_json::json!({"error": error_msg})),
                )
                .await;
            });
            AppError::DatabaseError(e)
        })?;

        let mut removed = vec![];
        for (id, filename) in &result {
            let storage_path = format!("{}/{}", self.config.storage.path, filename);
            let thumbnail_path = format!("{}/{}.jpg", self.config.storage.thumbnail_path, id);

            let file_removed = tokio::fs::remove_file(&storage_path).await.is_ok();
            let thumb_removed = tokio::fs::remove_file(&thumbnail_path).await.is_ok();

            if file_removed || thumb_removed {
                let _ = sqlx::query("DELETE FROM images WHERE id = $1")
                    .bind(id)
                    .execute(&self.pool)
                    .await;
                removed.push(filename.clone());
            }
        }

        info!(
            "Cleanup complete: {} images removed by {}",
            removed.len(),
            admin_username
        );

        // 记录审计日志
        let removed_count = removed.len();
        let pool = self.pool.clone();
        tokio::spawn(async move {
            let _ = log_audit(
                &pool,
                Some(admin_user_id),
                "cleanup_completed",
                "cleanup",
                None,
                None,
                Some(serde_json::json!({"removed_count": removed_count})),
            )
            .await;
        });

        Ok(removed)
    }

    /// 清理过期图片（移至回收站）
    pub async fn cleanup_expired_images(&self, admin_user_id: Uuid) -> Result<i64, AppError> {
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE images SET deleted_at = $1 WHERE expires_at < $1 AND deleted_at IS NULL",
        )
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to expire images: {}", e);
            let pool = self.pool.clone();
            let error_msg = e.to_string();
            tokio::spawn(async move {
                let _ = log_audit(
                    &pool,
                    Some(admin_user_id),
                    "expire_failed",
                    "expiry",
                    None,
                    None,
                    Some(serde_json::json!({"error": error_msg})),
                )
                .await;
            });
            AppError::DatabaseError(e)
        })?;

        let affected = result.rows_affected() as i64;
        if affected > 0 {
            info!("Expired images moved to trash: {}", affected);
        }

        // 记录审计日志
        let pool = self.pool.clone();
        tokio::spawn(async move {
            let _ = log_audit(
                &pool,
                Some(admin_user_id),
                "expire_completed",
                "expiry",
                None,
                None,
                Some(serde_json::json!({"affected_count": affected})),
            )
            .await;
        });

        Ok(affected)
    }

    /// 批准/拒绝图片
    pub async fn approve_images(&self, image_ids: &[Uuid], approved: bool) -> Result<(), AppError> {
        let status = if approved { "active" } else { "pending" };

        sqlx::query("UPDATE images SET status = $1 WHERE id = ANY($2)")
            .bind(status)
            .bind(image_ids)
            .execute(&self.pool)
            .await?;

        info!("Images approved: {:?} -> {}", image_ids, status);
        Ok(())
    }

    /// 获取所有用户
    pub async fn get_users(&self) -> Result<Vec<User>, AppError> {
        let users = sqlx::query_as(
            "SELECT id, username, password_hash, role, created_at FROM users ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }

    /// 更新用户角色
    pub async fn update_user_role(&self, user_id: Uuid, role: &str) -> Result<(), AppError> {
        if role != "admin" && role != "user" {
            return Err(AppError::InvalidPagination);
        }

        sqlx::query("UPDATE users SET role = $1 WHERE id = $2")
            .bind(role)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        info!("User role updated: {} -> {}", user_id, role);
        Ok(())
    }

    /// 获取审计日志
    pub async fn get_audit_logs(
        &self,
        page: i64,
        page_size: i64,
    ) -> Result<AuditLogResponse, AppError> {
        let offset = (page - 1) * page_size;

        let logs: Vec<AuditLog> =
            sqlx::query_as("SELECT * FROM audit_logs ORDER BY created_at DESC LIMIT $1 OFFSET $2")
                .bind(page_size)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_logs")
            .fetch_one(&self.pool)
            .await?;

        Ok(AuditLogResponse {
            data: logs,
            page: page as i32,
            page_size: page_size as i32,
            total,
        })
    }

    /// 获取系统统计
    pub async fn get_system_stats(&self) -> Result<SystemStats, AppError> {
        let now = Utc::now();
        let day_ago = now - chrono::Duration::days(1);
        let week_ago = now - chrono::Duration::days(7);

        let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        let total_images: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM images WHERE deleted_at IS NULL")
                .fetch_one(&self.pool)
                .await?;

        let total_storage: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(size), 0) FROM images WHERE deleted_at IS NULL",
        )
        .fetch_one(&self.pool)
        .await?;

        let total_views: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(views), 0) FROM images WHERE deleted_at IS NULL",
        )
        .fetch_one(&self.pool)
        .await?;

        let images_last_24h: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM images WHERE created_at > $1 AND deleted_at IS NULL",
        )
        .bind(day_ago)
        .fetch_one(&self.pool)
        .await?;

        let images_last_7d: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM images WHERE created_at > $1 AND deleted_at IS NULL",
        )
        .bind(week_ago)
        .fetch_one(&self.pool)
        .await?;

        Ok(SystemStats {
            total_users,
            total_images,
            total_storage,
            total_views,
            images_last_24h,
            images_last_7d,
        })
    }

    /// 获取设置列表
    pub async fn get_settings(&self) -> Result<Vec<Setting>, AppError> {
        let settings = sqlx::query_as("SELECT * FROM settings ORDER BY key")
            .fetch_all(&self.pool)
            .await?;

        Ok(settings)
    }

    /// 更新设置
    pub async fn update_setting(&self, key: &str, value: &str) -> Result<(), AppError> {
        let now = Utc::now();
        let exists =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM settings WHERE key = $1)")
                .bind(key)
                .fetch_one(&self.pool)
                .await?;

        if exists {
            sqlx::query("UPDATE settings SET value = $1, updated_at = $2 WHERE key = $3")
                .bind(value)
                .bind(now)
                .bind(key)
                .execute(&self.pool)
                .await?;
        } else {
            sqlx::query("INSERT INTO settings (key, value, updated_at) VALUES ($1, $2, $3)")
                .bind(key)
                .bind(value)
                .bind(now)
                .execute(&self.pool)
                .await?;
        }

        info!("Setting updated: {} = {}", key, value);
        Ok(())
    }

    /// 获取配置引用
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 获取数据库连接池引用
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}
