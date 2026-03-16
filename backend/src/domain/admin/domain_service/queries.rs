use chrono::{DateTime, Utc};
use tracing::info;
use uuid::Uuid;

use super::AdminDomainService;
use crate::db::DatabasePool;
use crate::error::AppError;
use crate::models::{
    AdminUserRecord, AdminUserSummary, AuditLog, AuditLogRecord, AuditLogResponse, SystemStats,
    UserRole,
};

const LAST_ADMIN_ROLE_CHANGE_ERROR: &str = "系统至少需要保留一个管理员账户";

#[derive(Debug)]
pub struct UserRoleUpdateResult {
    pub email: String,
    pub previous_role: UserRole,
    pub new_role: UserRole,
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
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, AdminUserRecord>(query)
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::MySql(pool) => {
                sqlx::query_as::<_, AdminUserRecord>(query)
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::Sqlite(pool) => {
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
            DatabasePool::MySql(pool) => {
                let mut tx = pool.begin().await?;
                let current = sqlx::query_as::<_, (String, String)>(
                    "SELECT email, role FROM users WHERE id = ? LIMIT 1 FOR UPDATE",
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
                     SET role = ?,
                         token_version = token_version + 1
                     WHERE id = ?",
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
            DatabasePool::Sqlite(pool) => {
                let mut tx = pool.begin().await?;
                let current = sqlx::query_as::<_, (String, String)>(
                    "SELECT email, role FROM users WHERE id = ?1 LIMIT 1",
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
                    let admin_count = sqlx::query_scalar::<_, i64>(
                        "SELECT COUNT(*) FROM users WHERE role = 'admin'",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    if admin_count <= 1 {
                        return Err(AppError::ValidationError(
                            LAST_ADMIN_ROLE_CHANGE_ERROR.to_string(),
                        ));
                    }
                }

                sqlx::query(
                    "UPDATE users
                     SET role = ?1,
                         token_version = token_version + 1
                     WHERE id = ?2",
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
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, AuditLogRecord>(
                    "SELECT * FROM audit_logs ORDER BY created_at DESC LIMIT $1 OFFSET $2",
                )
                .bind(page_size)
                .bind(offset)
                .fetch_all(pool)
                .await?
                .into_iter()
                .map(Into::into)
                .collect()
            }
            DatabasePool::MySql(pool) => {
                sqlx::query_as::<_, AuditLogRecord>(
                    "SELECT id, user_id, action, target_type, target_id, details, ip_address, created_at
                     FROM audit_logs
                     ORDER BY created_at DESC
                     LIMIT ? OFFSET ?",
                )
                .bind(page_size)
                .bind(offset)
                .fetch_all(pool)
                .await?
                .into_iter()
                .map(Into::into)
                .collect()
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
                sqlx::query_scalar::<_, i64>("SELECT CAST(COUNT(*) AS SIGNED) FROM audit_logs")
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
            }),
            DatabasePool::MySql(pool) => Ok(SystemStats {
                total_users: sqlx::query_scalar("SELECT CAST(COUNT(*) AS SIGNED) FROM users")
                    .fetch_one(pool)
                    .await?,
                total_images: sqlx::query_scalar(
                    "SELECT CAST(COUNT(*) AS SIGNED) FROM images WHERE status = 'active'",
                )
                    .fetch_one(pool)
                    .await?,
                total_storage: sqlx::query_scalar(
                    "SELECT CAST(COALESCE(SUM(size), 0) AS SIGNED) FROM images WHERE status = 'active'",
                )
                .fetch_one(pool)
                .await?,
                total_views: sqlx::query_scalar(
                    "SELECT CAST(COALESCE(SUM(views), 0) AS SIGNED) FROM images WHERE status = 'active'",
                )
                .fetch_one(pool)
                .await?,
                images_last_24h: sqlx::query_scalar(
                    "SELECT CAST(COUNT(*) AS SIGNED) FROM audit_logs WHERE action = 'image.upload' AND created_at > ?",
                )
                .bind(day_ago)
                .fetch_one(pool)
                .await?,
                images_last_7d: sqlx::query_scalar(
                    "SELECT CAST(COUNT(*) AS SIGNED) FROM audit_logs WHERE action = 'image.upload' AND created_at > ?",
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
                    "SELECT COUNT(*) FROM images WHERE status = 'active'",
                )
                    .fetch_one(pool)
                    .await?,
                total_storage: sqlx::query_scalar(
                    "SELECT COALESCE(SUM(size), 0) FROM images WHERE status = 'active'",
                )
                    .fetch_one(pool)
                    .await?,
                total_views: sqlx::query_scalar(
                    "SELECT COALESCE(SUM(views), 0) FROM images WHERE status = 'active'",
                )
                    .fetch_one(pool)
                    .await?,
                images_last_24h: sqlx::query_scalar(
                    "SELECT COUNT(*) FROM audit_logs WHERE action = 'image.upload' AND created_at > ?1",
                )
                .bind(day_ago)
                .fetch_one(pool)
                .await?,
                images_last_7d: sqlx::query_scalar(
                    "SELECT COUNT(*) FROM audit_logs WHERE action = 'image.upload' AND created_at > ?1",
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
        id: id.to_string(),
        user_id: user_id.map(|id| id.to_string()),
        action,
        target_type,
        target_id: target_id.map(|id| id.to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DatabaseKind};
    use crate::db::run_migrations;
    use crate::models::{ComponentStatus, UserRole};
    use crate::runtime_settings::RuntimeSettings;
    use crate::storage_backend::StorageManager;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::sync::Arc;

    async fn sqlite_admin_service() -> (tempfile::TempDir, sqlx::SqlitePool, AdminDomainService) {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let database_path = temp_dir.path().join("admin.db");
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

        let mut config = Config::default();
        config.database.kind = DatabaseKind::Sqlite;
        config.database.url = database_path.to_string_lossy().into_owned();

        let database = DatabasePool::Sqlite(pool.clone());
        run_migrations(&database)
            .await
            .expect("migrations should succeed");

        let storage_manager =
            Arc::new(StorageManager::new(RuntimeSettings::from_defaults(&config)));
        let service = AdminDomainService::new(
            database,
            None,
            ComponentStatus::disabled("cache disabled"),
            config,
            storage_manager,
        );

        (temp_dir, pool, service)
    }

    async fn insert_user(pool: &sqlx::SqlitePool, id: Uuid, email: &str, role: &str) {
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, role, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(id)
        .bind(email)
        .bind("password-hash")
        .bind(role)
        .bind(Utc::now())
        .execute(pool)
        .await
        .expect("user should be inserted");
    }

    async fn insert_image(
        pool: &sqlx::SqlitePool,
        user_id: Uuid,
        filename: &str,
        hash: &str,
        status: &str,
        created_at: DateTime<Utc>,
    ) {
        sqlx::query(
            "INSERT INTO images (id, user_id, filename, thumbnail, size, hash, format, views, status, expires_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(filename)
        .bind(None::<String>)
        .bind(128_i64)
        .bind(hash)
        .bind("png")
        .bind(0_i64)
        .bind(status)
        .bind(None::<DateTime<Utc>>)
        .bind(created_at)
        .execute(pool)
        .await
        .expect("image should be inserted");
    }

    async fn insert_audit_log(
        pool: &sqlx::SqlitePool,
        user_id: Option<Uuid>,
        action: &str,
        target_type: &str,
        target_id: Option<Uuid>,
        created_at: DateTime<Utc>,
    ) {
        sqlx::query(
            "INSERT INTO audit_logs (id, user_id, action, target_type, target_id, details, ip_address, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(action)
        .bind(target_type)
        .bind(target_id)
        .bind(None::<String>)
        .bind(None::<String>)
        .bind(created_at)
        .execute(pool)
        .await
        .expect("audit log should be inserted");
    }

    #[tokio::test]
    async fn update_user_role_rejects_demoting_last_admin() {
        let (_temp_dir, pool, service) = sqlite_admin_service().await;
        let admin_id = Uuid::new_v4();
        insert_user(&pool, admin_id, "admin@example.com", "admin").await;

        let error = service
            .update_user_role(admin_id, UserRole::User)
            .await
            .expect_err("demoting the last admin should fail");

        assert!(matches!(
            error,
            AppError::ValidationError(ref message) if message == LAST_ADMIN_ROLE_CHANGE_ERROR
        ));
    }

    #[tokio::test]
    async fn update_user_role_bumps_token_version_when_role_changes() {
        let (_temp_dir, pool, service) = sqlite_admin_service().await;
        let admin_id = Uuid::new_v4();
        let second_admin_id = Uuid::new_v4();
        insert_user(&pool, admin_id, "admin@example.com", "admin").await;
        insert_user(&pool, second_admin_id, "second-admin@example.com", "admin").await;

        let result = service
            .update_user_role(admin_id, UserRole::User)
            .await
            .expect("role update should succeed");

        assert!(result.changed);
        assert_eq!(result.previous_role, UserRole::Admin);
        assert_eq!(result.new_role, UserRole::User);

        let (role, token_version) = sqlx::query_as::<_, (String, i64)>(
            "SELECT role, token_version FROM users WHERE id = ?1",
        )
        .bind(admin_id)
        .fetch_one(&pool)
        .await
        .expect("updated user should exist");

        assert_eq!(role, "user");
        assert_eq!(token_version, 1);
    }

    #[tokio::test]
    async fn system_stats_count_recent_uploads_even_after_soft_delete() {
        let (_temp_dir, pool, service) = sqlite_admin_service().await;
        let user_id = Uuid::new_v4();
        let image_id = Uuid::new_v4();
        insert_user(&pool, user_id, "admin@example.com", "admin").await;
        let uploaded_at = Utc::now();
        insert_image(
            &pool,
            user_id,
            "deleted.png",
            &"d".repeat(64),
            "active",
            uploaded_at,
        )
        .await;
        sqlx::query("UPDATE images SET id = ?1 WHERE filename = ?2")
            .bind(image_id)
            .bind("deleted.png")
            .execute(&pool)
            .await
            .expect("image id should be updated");
        insert_audit_log(
            &pool,
            Some(user_id),
            "image.upload",
            "image",
            Some(image_id),
            uploaded_at,
        )
        .await;
        sqlx::query("DELETE FROM images WHERE id = ?1")
            .bind(image_id)
            .execute(&pool)
            .await
            .expect("image should be hard deleted");

        let stats = service
            .get_system_stats()
            .await
            .expect("system stats should load");

        assert_eq!(stats.total_users, 1);
        assert_eq!(stats.total_images, 0);
        assert_eq!(stats.total_storage, 0);
        assert_eq!(stats.images_last_24h, 1);
        assert_eq!(stats.images_last_7d, 1);
    }
}
