use redis::aio::ConnectionManager;
use sqlx::{PgPool, Executor, Row};
use crate::auth::AuthService;
use crate::cache::Cache;
use crate::config::Config;
use crate::domain::auth::DefaultAuthDomainService;
use crate::domain::image::ImageDomainService;
use crate::domain::image::repository::{PostgresImageRepository, PostgresCategoryRepository};
use crate::domain::admin::AdminDomainService;
use crate::file_queue::FileSaveQueue;
use crate::image_processor::ImageProcessor;
use uuid::Uuid;
use tracing::{info, error};
use std::time::Instant;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub redis: ConnectionManager,
    pub config: Config,
    pub auth: AuthService,
    pub auth_domain_service: Option<Arc<DefaultAuthDomainService>>,
    pub image_domain_service: Option<Arc<ImageDomainService<PostgresImageRepository, PostgresCategoryRepository>>>,
    pub admin_domain_service: Option<Arc<AdminDomainService>>,
    pub image_processor: ImageProcessor,
    pub file_save_queue: Arc<FileSaveQueue>,
    pub started_at: Instant,
}

// 管理员用户固定 ID（每次启动时使用相同 ID）
pub const ADMIN_USER_ID: Uuid = uuid::uuid!("00000000-0000-0000-0000-000000000001");

// ==================== 数据库架构定义 ====================

/// 数据库初始化 SQL
const SCHEMA_SQL: &str = r#"
-- 用户表（简化为单管理员模式）
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    username VARCHAR(50) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(20) DEFAULT 'admin',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 图片表
CREATE TABLE IF NOT EXISTS images (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    category_id UUID,
    filename VARCHAR(255) NOT NULL,
    thumbnail VARCHAR(255),
    original_filename VARCHAR(255),
    size BIGINT NOT NULL,
    hash VARCHAR(64) NOT NULL,
    format VARCHAR(20),
    views INTEGER DEFAULT 0,
    status VARCHAR(20) DEFAULT 'active',
    expires_at TIMESTAMP WITH TIME ZONE,
    deleted_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 图片标签表
CREATE TABLE IF NOT EXISTS image_tags (
    image_id UUID REFERENCES images(id) ON DELETE CASCADE,
    tag VARCHAR(50) NOT NULL
);

-- 审计日志表
CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action VARCHAR(50) NOT NULL,
    target_type VARCHAR(20),
    target_id UUID,
    details JSONB,
    ip_address VARCHAR(45),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 设置表
CREATE TABLE IF NOT EXISTS settings (
    key VARCHAR(50) PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 基础索引
CREATE INDEX IF NOT EXISTS idx_images_user_id ON images(user_id);
CREATE INDEX IF NOT EXISTS idx_images_category_id ON images(category_id);
CREATE INDEX IF NOT EXISTS idx_images_created_at ON images(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_images_views ON images(views DESC);
CREATE INDEX IF NOT EXISTS idx_images_size ON images(size);
CREATE INDEX IF NOT EXISTS idx_images_deleted_at ON images(deleted_at);
CREATE INDEX IF NOT EXISTS idx_images_hash ON images(hash);
CREATE INDEX IF NOT EXISTS idx_images_status ON images(status);
CREATE INDEX IF NOT EXISTS idx_images_expires_at ON images(expires_at);
CREATE INDEX IF NOT EXISTS idx_image_tags_image_id ON image_tags(image_id);
CREATE INDEX IF NOT EXISTS idx_image_tags_tag ON image_tags(tag);
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);

-- 复合索引优化常用查询
CREATE INDEX IF NOT EXISTS idx_images_user_status_created ON images(user_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_images_status_expires ON images(status, expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_images_hash_user_deleted ON images(hash, user_id, deleted_at) WHERE deleted_at IS NULL;

-- 启用模糊搜索（pg_trgm 扩展）
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX IF NOT EXISTS idx_images_filename_trgm ON images USING gin (filename gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_images_original_filename_trgm ON images USING gin (original_filename gin_trgm_ops) WHERE original_filename IS NOT NULL;

-- 分页索引优化
CREATE INDEX IF NOT EXISTS idx_images_user_status_partial ON images(user_id, status) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_images_user_category_status_partial ON images(user_id, category_id, status) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_images_user_status_created_partial ON images(user_id, status, created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_images_user_status_expires_partial ON images(user_id, status, expires_at) WHERE deleted_at IS NULL AND status = 'active';

-- 复合索引引用列
CREATE INDEX IF NOT EXISTS idx_images_user_category_deleted ON images(user_id, category_id, deleted_at);
"#;

/// 初始化数据库架构
pub async fn init_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
    pool.execute(SCHEMA_SQL).await?;
    info!("Database schema initialized successfully");
    Ok(())
}

/// 创建或更新管理员账户
pub async fn create_admin_account(pool: &PgPool, config: &Config) -> Result<(String, String), anyhow::Error> {
    let admin_username = &config.admin.username;
    let admin_password = &config.admin.password;
    let password_hash = AuthService::hash_password(admin_password)?;

    // 使用 INSERT ... ON CONFLICT DO UPDATE 来创建或更新管理员
    let result = sqlx::query(
        "INSERT INTO users (id, username, password_hash, role, created_at)
         VALUES ($1, $2, $3, 'admin', NOW())
         ON CONFLICT (username) DO UPDATE
         SET password_hash = $3
         RETURNING username"
    )
    .bind(ADMIN_USER_ID)
    .bind(admin_username)
    .bind(&password_hash)
    .fetch_optional(pool)
    .await;

    match result {
        Ok(Some(row)) => {
            let username: String = row.get("username");
            info!("Admin account initialized: {}", username);
            Ok((username, admin_password.clone()))
        }
        Ok(None) => {
            error!("Failed to create admin account: no result returned");
            Err(anyhow::anyhow!("Failed to create admin account: no result returned"))
        }
        Err(e) => {
            error!("Failed to create admin account: {}", e);
            Err(anyhow::anyhow!("Failed to create admin account: {}", e))
        }
    }
}

/// 打印管理员账户信息到日志
pub fn log_admin_credentials(config: &Config) {
    info!("========================================");
    info!("        Admin Account");
    info!("========================================");
    info!("        Username: {}", config.admin.username);
    info!("        Password: {}", config.admin.password);
    info!("========================================");
    info!("");
    info!("You can login with these credentials.");
    info!("To change credentials, use environment variables:");
    info!("  - ADMIN_USERNAME=your_username");
    info!("  - ADMIN_PASSWORD=your_password");
    info!("");
}

impl AppState {
    /// 清除用户图片缓存
    pub async fn invalidate_user_image_cache(&self, user_id: Uuid) -> Result<(), anyhow::Error> {
        let mut redis = self.redis.clone();
        Cache::del_pattern(&mut redis, &crate::cache::ImageCache::images_invalidate(user_id)).await
    }

    /// 清除用户分类缓存
    pub async fn invalidate_user_category_cache(&self, user_id: Uuid) -> Result<(), anyhow::Error> {
        let mut redis = self.redis.clone();
        Cache::del_pattern(&mut redis, &crate::cache::ImageCache::categories_invalidate(user_id)).await
    }

    /// 清除用户所有缓存
    pub async fn invalidate_user_caches(&self, user_id: Uuid) -> Result<(), anyhow::Error> {
        self.invalidate_user_image_cache(user_id).await?;
        self.invalidate_user_category_cache(user_id).await?;
        Ok(())
    }
}
