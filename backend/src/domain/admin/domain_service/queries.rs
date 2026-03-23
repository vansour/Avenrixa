use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use super::AdminDomainService;
use crate::db::DatabasePool;
use crate::error::AppError;
use crate::models::{
    AdminUserRecord, AdminUserSummary, AuditLogRecord, AuditLogResponse, RuntimeBacklogMetrics,
    RuntimeObservabilitySnapshot, SystemStats, UserRole,
};

const LAST_ADMIN_ROLE_CHANGE_ERROR: &str = "系统至少需要保留一个管理员账户";

#[derive(Debug)]
pub struct UserRoleUpdateResult {
    pub email: String,
    pub previous_role: UserRole,
    pub new_role: UserRole,
    pub changed: bool,
}

impl AdminDomainService {
    pub async fn get_users(&self) -> Result<Vec<AdminUserSummary>, AppError> {
        let query = "SELECT id, email, role, created_at FROM users ORDER BY created_at DESC";
        let users = match &self.database {
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, AdminUserRecord>(query)
                    .fetch_all(pool)
                    .await?
            }
        };

        Ok(users.into_iter().map(Into::into).collect())
    }

    pub async fn update_user_role(
        &self,
        user_id: Uuid,
        role: UserRole,
    ) -> Result<UserRoleUpdateResult, AppError> {
        if !matches!(role, UserRole::Admin | UserRole::User) {
            return Err(AppError::ValidationError(
                "角色必须是 admin 或 user".to_string(),
            ));
        }

        let result = match &self.database {
            DatabasePool::Postgres(pool) => {
                let mut tx = pool.begin().await?;
                let current = sqlx::query_as::<_, (String, String)>(
                    "SELECT email, role FROM users WHERE id = $1 LIMIT 1 FOR UPDATE",
                )
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(AppError::UserNotFound)?;

                let (email, previous_role) = current;
                let previous_role = UserRole::parse(&previous_role);
                if previous_role == role {
                    return Ok(UserRoleUpdateResult {
                        email,
                        previous_role,
                        new_role: role,
                        changed: false,
                    });
                }

                if previous_role.is_admin() && role == UserRole::User {
                    let admin_ids = sqlx::query_scalar::<_, Uuid>(
                        "SELECT id FROM users WHERE role = 'admin' FOR UPDATE",
                    )
                    .fetch_all(&mut *tx)
                    .await?;
                    if admin_ids.len() <= 1 {
                        return Err(AppError::ValidationError(
                            LAST_ADMIN_ROLE_CHANGE_ERROR.to_string(),
                        ));
                    }
                }

                sqlx::query(
                    "UPDATE users
                     SET role = $1,
                         token_version = token_version + 1
                     WHERE id = $2",
                )
                .bind(role.as_str())
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;

                UserRoleUpdateResult {
                    email,
                    previous_role,
                    new_role: role,
                    changed: true,
                }
            }
        };

        if result.changed {
            info!("User role updated: {} -> {}", user_id, role.as_str());
        }
        Ok(result)
    }

    pub async fn get_audit_logs(
        &self,
        page: i64,
        page_size: i64,
    ) -> Result<AuditLogResponse, AppError> {
        let offset = (page - 1) * page_size;

        let logs = match &self.database {
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, AuditLogRecord>(
                "SELECT * FROM audit_logs ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            )
            .bind(page_size)
            .bind(offset)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
        };

        let total = match &self.database {
            DatabasePool::Postgres(pool) => {
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
        let runtime = self.collect_runtime_observability().await?;

        match &self.database {
            DatabasePool::Postgres(pool) => Ok(SystemStats {
                total_users: sqlx::query_scalar("SELECT COUNT(*) FROM users")
                    .fetch_one(pool)
                    .await?,
                total_images: sqlx::query_scalar(
                    "SELECT COUNT(*) FROM images WHERE status = 'active'",
                )
                    .fetch_one(pool)
                    .await?,
                total_storage: sqlx::query_scalar(
                    "SELECT COALESCE(SUM(size), 0)::BIGINT FROM images WHERE status = 'active'",
                )
                    .fetch_one(pool)
                    .await?,
                total_views: sqlx::query_scalar(
                    "SELECT COALESCE(SUM(views), 0)::BIGINT FROM images WHERE status = 'active'",
                )
                    .fetch_one(pool)
                    .await?,
                images_last_24h: sqlx::query_scalar(
                    "SELECT COUNT(*) FROM audit_logs WHERE action = 'image.upload' AND created_at > $1",
                )
                    .bind(day_ago)
                    .fetch_one(pool)
                    .await?,
                images_last_7d: sqlx::query_scalar(
                    "SELECT COUNT(*) FROM audit_logs WHERE action = 'image.upload' AND created_at > $1",
                )
                    .bind(week_ago)
                    .fetch_one(pool)
                    .await?,
                runtime,
            }),
        }
    }

    pub(super) async fn collect_runtime_observability(
        &self,
    ) -> Result<RuntimeObservabilitySnapshot, AppError> {
        let backlog = self.storage_cleanup_backlog().await?;
        Ok(self.observability.snapshot(backlog))
    }

    async fn storage_cleanup_backlog(&self) -> Result<RuntimeBacklogMetrics, AppError> {
        match &self.database {
            DatabasePool::Postgres(pool) => Ok(RuntimeBacklogMetrics {
                storage_cleanup_pending: sqlx::query_scalar(
                    "SELECT COUNT(*) FROM storage_cleanup_jobs",
                )
                .fetch_one(pool)
                .await?,
                storage_cleanup_retrying: sqlx::query_scalar(
                    "SELECT COUNT(*) FROM storage_cleanup_jobs WHERE attempts > 0 OR last_error IS NOT NULL",
                )
                .fetch_one(pool)
                .await?,
            }),
        }
    }
}
