use chrono::{DateTime, Utc};
use tracing::info;
use uuid::Uuid;

use super::AdminDomainService;
use crate::db::DatabasePool;
use crate::error::AppError;
use crate::models::{AdminUserSummary, AuditLog, AuditLogResponse, SystemStats};

pub struct UserRoleUpdateResult {
    pub email: String,
    pub previous_role: String,
    pub new_role: String,
    pub changed: bool,
}

type SqliteAuditLogRow = (
    Uuid,
    Option<Uuid>,
    String,
    String,
    Option<Uuid>,
    Option<String>,
    Option<String>,
    DateTime<Utc>,
);

impl AdminDomainService {
    pub async fn get_users(&self) -> Result<Vec<AdminUserSummary>, AppError> {
        let query = "SELECT id, email, role, created_at FROM users ORDER BY created_at DESC";
        let users = match &self.database {
            DatabasePool::Postgres(pool) => sqlx::query_as(query).fetch_all(pool).await?,
            DatabasePool::MySql(pool) => sqlx::query_as(query).fetch_all(pool).await?,
            DatabasePool::Sqlite(pool) => sqlx::query_as(query).fetch_all(pool).await?,
        };

        Ok(users)
    }

    pub async fn update_user_role(
        &self,
        user_id: Uuid,
        role: &str,
    ) -> Result<UserRoleUpdateResult, AppError> {
        if role != "admin" && role != "user" {
            return Err(AppError::ValidationError(
                "角色必须是 admin 或 user".to_string(),
            ));
        }

        let current = match &self.database {
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, (String, String)>(
                    "SELECT email, role FROM users WHERE id = $1 LIMIT 1",
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySql(pool) => {
                sqlx::query_as::<_, (String, String)>(
                    "SELECT email, role FROM users WHERE id = ? LIMIT 1",
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, (String, String)>(
                    "SELECT email, role FROM users WHERE id = ?1 LIMIT 1",
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await?
            }
        }
        .ok_or(AppError::UserNotFound)?;

        let (email, previous_role) = current;
        let changed = previous_role != role;

        if !changed {
            return Ok(UserRoleUpdateResult {
                email,
                previous_role: previous_role.clone(),
                new_role: role.to_string(),
                changed: false,
            });
        }

        match &self.database {
            DatabasePool::Postgres(pool) => {
                sqlx::query("UPDATE users SET role = $1 WHERE id = $2")
                    .bind(role)
                    .bind(user_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySql(pool) => {
                sqlx::query("UPDATE users SET role = ? WHERE id = ?")
                    .bind(role)
                    .bind(user_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("UPDATE users SET role = ?1 WHERE id = ?2")
                    .bind(role)
                    .bind(user_id)
                    .execute(pool)
                    .await?;
            }
        }

        info!("User role updated: {} -> {}", user_id, role);
        Ok(UserRoleUpdateResult {
            email,
            previous_role,
            new_role: role.to_string(),
            changed: true,
        })
    }

    pub async fn get_audit_logs(
        &self,
        page: i64,
        page_size: i64,
    ) -> Result<AuditLogResponse, AppError> {
        let offset = (page - 1) * page_size;

        let logs = match &self.database {
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, AuditLog>(
                    "SELECT * FROM audit_logs ORDER BY created_at DESC LIMIT $1 OFFSET $2",
                )
                .bind(page_size)
                .bind(offset)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySql(pool) => {
                sqlx::query_as::<_, AuditLog>(
                    "SELECT id, user_id, action, target_type, target_id, details, ip_address, created_at
                     FROM audit_logs
                     ORDER BY created_at DESC
                     LIMIT ? OFFSET ?",
                )
                .bind(page_size)
                .bind(offset)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query_as::<_, SqliteAuditLogRow>(
                    "SELECT id, user_id, action, target_type, target_id, details, ip_address, created_at
                     FROM audit_logs
                     ORDER BY created_at DESC
                     LIMIT ?1 OFFSET ?2",
                )
                .bind(page_size)
                .bind(offset)
                .fetch_all(pool)
                .await?;
                rows.into_iter().map(map_sqlite_audit_log).collect()
            }
        };

        let total = match &self.database {
            DatabasePool::Postgres(pool) => {
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM audit_logs")
                    .fetch_one(pool)
                    .await?
            }
            DatabasePool::MySql(pool) => {
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM audit_logs")
                    .fetch_one(pool)
                    .await?
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM audit_logs")
                    .fetch_one(pool)
                    .await?
            }
        };

        Ok(AuditLogResponse {
            data: logs,
            page: page as i32,
            page_size: page_size as i32,
            total,
        })
    }

    pub async fn get_system_stats(&self) -> Result<SystemStats, AppError> {
        let now = Utc::now();
        let day_ago = now - chrono::Duration::days(1);
        let week_ago = now - chrono::Duration::days(7);

        match &self.database {
            DatabasePool::Postgres(pool) => Ok(SystemStats {
                total_users: sqlx::query_scalar("SELECT COUNT(*) FROM users")
                    .fetch_one(pool)
                    .await?,
                total_images: sqlx::query_scalar(
                    "SELECT COUNT(*) FROM images WHERE deleted_at IS NULL",
                )
                .fetch_one(pool)
                .await?,
                total_storage: sqlx::query_scalar(
                    "SELECT COALESCE(SUM(size), 0)::BIGINT FROM images WHERE deleted_at IS NULL",
                )
                .fetch_one(pool)
                .await?,
                total_views: sqlx::query_scalar(
                    "SELECT COALESCE(SUM(views), 0)::BIGINT FROM images WHERE deleted_at IS NULL",
                )
                .fetch_one(pool)
                .await?,
                images_last_24h: sqlx::query_scalar(
                    "SELECT COUNT(*) FROM images WHERE created_at > $1 AND deleted_at IS NULL",
                )
                .bind(day_ago)
                .fetch_one(pool)
                .await?,
                images_last_7d: sqlx::query_scalar(
                    "SELECT COUNT(*) FROM images WHERE created_at > $1 AND deleted_at IS NULL",
                )
                .bind(week_ago)
                .fetch_one(pool)
                .await?,
            }),
            DatabasePool::MySql(pool) => Ok(SystemStats {
                total_users: sqlx::query_scalar("SELECT COUNT(*) FROM users")
                    .fetch_one(pool)
                    .await?,
                total_images: sqlx::query_scalar(
                    "SELECT COUNT(*) FROM images WHERE deleted_at IS NULL",
                )
                .fetch_one(pool)
                .await?,
                total_storage: sqlx::query_scalar(
                    "SELECT COALESCE(SUM(size), 0) FROM images WHERE deleted_at IS NULL",
                )
                .fetch_one(pool)
                .await?,
                total_views: sqlx::query_scalar(
                    "SELECT COALESCE(SUM(views), 0) FROM images WHERE deleted_at IS NULL",
                )
                .fetch_one(pool)
                .await?,
                images_last_24h: sqlx::query_scalar(
                    "SELECT COUNT(*) FROM images WHERE created_at > ? AND deleted_at IS NULL",
                )
                .bind(day_ago)
                .fetch_one(pool)
                .await?,
                images_last_7d: sqlx::query_scalar(
                    "SELECT COUNT(*) FROM images WHERE created_at > ? AND deleted_at IS NULL",
                )
                .bind(week_ago)
                .fetch_one(pool)
                .await?,
            }),
            DatabasePool::Sqlite(pool) => Ok(SystemStats {
                total_users: sqlx::query_scalar("SELECT COUNT(*) FROM users")
                    .fetch_one(pool)
                    .await?,
                total_images: sqlx::query_scalar(
                    "SELECT COUNT(*) FROM images WHERE deleted_at IS NULL",
                )
                .fetch_one(pool)
                .await?,
                total_storage: sqlx::query_scalar(
                    "SELECT COALESCE(SUM(size), 0) FROM images WHERE deleted_at IS NULL",
                )
                .fetch_one(pool)
                .await?,
                total_views: sqlx::query_scalar(
                    "SELECT COALESCE(SUM(views), 0) FROM images WHERE deleted_at IS NULL",
                )
                .fetch_one(pool)
                .await?,
                images_last_24h: sqlx::query_scalar(
                    "SELECT COUNT(*) FROM images WHERE created_at > ?1 AND deleted_at IS NULL",
                )
                .bind(day_ago)
                .fetch_one(pool)
                .await?,
                images_last_7d: sqlx::query_scalar(
                    "SELECT COUNT(*) FROM images WHERE created_at > ?1 AND deleted_at IS NULL",
                )
                .bind(week_ago)
                .fetch_one(pool)
                .await?,
            }),
        }
    }
}

fn map_sqlite_audit_log(
    (id, user_id, action, target_type, target_id, details, ip_address, created_at): SqliteAuditLogRow,
) -> AuditLog {
    AuditLog {
        id,
        user_id,
        action,
        target_type,
        target_id,
        details: details.and_then(parse_audit_details),
        ip_address,
        created_at,
    }
}

fn parse_audit_details(raw: String) -> Option<serde_json::Value> {
    serde_json::from_str(&raw)
        .ok()
        .or(Some(serde_json::Value::String(raw)))
}
