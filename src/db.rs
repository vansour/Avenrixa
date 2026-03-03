use redis::aio::ConnectionManager;
use sqlx::{PgPool, Executor};
use crate::auth::AuthService;
use crate::cache::Cache;
use crate::config::Config;
use crate::image_processor::ImageProcessor;
use uuid::Uuid;
use tracing::{info, warn, error};
use std::time::Instant;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub redis: ConnectionManager,
    pub config: Config,
    pub auth: AuthService,
    pub image_processor: ImageProcessor,
    pub started_at: Instant,
}

pub async fn init_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
    pool.execute(include_str!("schema.sql")).await?;
    Ok(())
}

/// 检查管理员账号是否存在
pub async fn check_admin_exists(pool: &PgPool, username: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query_as::<_, (Uuid,)>(
        "SELECT id FROM users WHERE username = $1"
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;
    Ok(result.is_some())
}

/// 创建默认管理员账号
pub async fn create_default_admin(pool: &PgPool) -> Result<(String, String), anyhow::Error> {
    const ADMIN_USERNAME: &str = "admin";
    const ADMIN_PASSWORD: &str = "admin";

    // 检查是否已存在
    if check_admin_exists(pool, ADMIN_USERNAME).await? {
        return Err(anyhow::anyhow!("Admin account already exists"));
    }

    let password_hash = AuthService::hash_password(ADMIN_PASSWORD)?;

    // 插入管理员记录
    let user_id = Uuid::new_v4();
    let result = sqlx::query(
        "INSERT INTO users (id, username, password_hash, role, created_at) VALUES ($1, $2, $3, 'admin', NOW())"
    )
    .bind(user_id)
    .bind(ADMIN_USERNAME)
    .bind(&password_hash)
    .execute(pool)
    .await;

    match result {
        Ok(_) => {
            info!("Default admin account created successfully");
            Ok((ADMIN_USERNAME.to_string(), ADMIN_PASSWORD.to_string()))
        }
        Err(e) => {
            error!("Failed to create default admin account: {}", e);
            Err(anyhow::anyhow!("Failed to create admin account: {}", e))
        }
    }
}

/// 输出管理员账号信息到日志（用于启动时显示）
pub async fn log_admin_credentials(pool: &PgPool) -> Result<(), anyhow::Error> {
    let result = sqlx::query_as::<_, (String,)>(
        "SELECT username FROM users WHERE role = 'admin' LIMIT 1"
    )
    .fetch_optional(pool)
    .await;

    match result {
        Ok(Some((username,))) => {
            info!("========================================");
            info!("        Admin Credentials");
            info!("========================================");
            info!("        Username: {}", username);
            info!("        Password: admin");
            info!("========================================");

            Ok(())
        }
        Ok(None) => {
            warn!("No admin account found in database");
            Err(anyhow::anyhow!("No admin account found in database"))
        }
        Err(e) => Err(anyhow::anyhow!("Failed to query admin credentials: {}", e))
    }
}

impl AppState {
    /// 清除用户图片缓存
    pub async fn invalidate_user_image_cache(&self, user_id: Uuid) -> Result<(), anyhow::Error> {
        let mut redis = self.redis.clone();
        Cache::del_pattern(&mut redis, &format!("images:list:{}:*", user_id)).await
    }

    /// 清除用户分类缓存
    pub async fn invalidate_user_category_cache(&self, _user_id: Uuid) -> Result<(), anyhow::Error> {
        let mut redis = self.redis.clone();
        Cache::del_pattern(&mut redis, "categories:list:*").await
    }

    /// 清除用户所有缓存
    pub async fn invalidate_user_caches(&self, user_id: Uuid) -> Result<(), anyhow::Error> {
        self.invalidate_user_image_cache(user_id).await?;
        self.invalidate_user_category_cache(user_id).await?;
        Ok(())
    }
}
