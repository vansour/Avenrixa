use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use super::AdminDomainService;
use crate::error::AppError;
use crate::models::{AdminUserSummary, AuditLog, AuditLogResponse, SystemStats};

impl AdminDomainService {
    pub async fn get_users(&self) -> Result<Vec<AdminUserSummary>, AppError> {
        let users = sqlx::query_as(
            "SELECT id, username, role, created_at FROM users ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }

    pub async fn update_user_role(&self, user_id: Uuid, role: &str) -> Result<(), AppError> {
        if role != "admin" && role != "user" {
            return Err(AppError::ValidationError(
                "角色必须是 admin 或 user".to_string(),
            ));
        }

        sqlx::query("UPDATE users SET role = $1 WHERE id = $2")
            .bind(role)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        info!("User role updated: {} -> {}", user_id, role);
        Ok(())
    }

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
            "SELECT COALESCE(SUM(size), 0)::BIGINT FROM images WHERE deleted_at IS NULL",
        )
        .fetch_one(&self.pool)
        .await?;

        let total_views: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(views), 0)::BIGINT FROM images WHERE deleted_at IS NULL",
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
}
