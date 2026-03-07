use redis::aio::ConnectionManager;
use sqlx::{PgPool, Executor};
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
use tracing::{info, warn, error};
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

// ==================== 数据库架构定义 ====================

/// 数据库初始化 SQL
const SCHEMA_SQL: &str = r#"
-- 用户表
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    username VARCHAR(50) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(20) DEFAULT 'user',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 密码重置令牌表
CREATE TABLE IF NOT EXISTS password_reset_tokens (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    token VARCHAR(255) UNIQUE NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    used_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_user_active_token UNIQUE (user_id, used_at)
);

-- 速率限制表（防止密码重置暴力破解）
CREATE TABLE IF NOT EXISTS rate_limits (
    id UUID PRIMARY KEY,
    ip_address VARCHAR(45) NOT NULL,
    action_type VARCHAR(50) NOT NULL,
    request_count INTEGER NOT NULL DEFAULT 1,
    window_start TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_reset TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 速率限制索引
CREATE INDEX IF NOT EXISTS idx_rate_limits_ip_action ON rate_limits(ip_address, action_type);
CREATE INDEX IF NOT EXISTS idx_rate_limits_window ON rate_limits(window_start);

-- 令牌撤销表（用于主动撤销 JWT 令牌）
CREATE TABLE IF NOT EXISTS token_revoked (
    id UUID PRIMARY KEY,
    token_id VARCHAR(255) UNIQUE NOT NULL,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    revoked_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    reason VARCHAR(50),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 令牌撤销索引
CREATE INDEX IF NOT EXISTS idx_token_revoked_token_id ON token_revoked(token_id);
CREATE INDEX IF NOT EXISTS idx_token_revoked_user_id ON token_revoked(user_id);
CREATE INDEX IF NOT EXISTS idx_token_revoked_expires_at ON token_revoked(expires_at);

-- 分类表
CREATE TABLE IF NOT EXISTS categories (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(50) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 图片表
CREATE TABLE IF NOT EXISTS images (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    category_id UUID REFERENCES categories(id) ON DELETE SET NULL,
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
CREATE INDEX IF NOT EXISTS idx_categories_user_id ON categories(user_id);
CREATE INDEX IF NOT EXISTS idx_image_tags_image_id ON image_tags(image_id);
CREATE INDEX IF NOT EXISTS idx_image_tags_tag ON image_tags(tag);
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);

-- 密码重置令牌索引
CREATE INDEX IF NOT EXISTS idx_password_reset_tokens_token ON password_reset_tokens(token);
CREATE INDEX IF NOT EXISTS idx_password_reset_tokens_user_id ON password_reset_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_password_reset_tokens_expires_at ON password_reset_tokens(expires_at);

-- 复合索引优化常用查询
CREATE INDEX IF NOT EXISTS idx_images_user_status_created ON images(user_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_images_status_expires ON images(status, expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_images_hash_user_deleted ON images(hash, user_id, deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_categories_user_name ON categories(user_id, name);

-- 启用模糊搜索（pg_trgm 扩展）
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX IF NOT EXISTS idx_images_filename_trgm ON images USING gin (filename gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_images_original_filename_trgm ON images USING gin (original_filename gin_trgm_ops) WHERE original_filename IS NOT NULL;

-- 分页索引优化：仅对活跃记录（deleted_at IS NULL）创建索引
-- 这些索引更小，查询更快，且在软删除时自动失效
CREATE INDEX IF NOT EXISTS idx_images_user_status_partial ON images(user_id, status) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_images_user_category_status_partial ON images(user_id, category_id, status) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_images_user_status_created_partial ON images(user_id, status, created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_images_user_status_expires_partial ON images(user_id, status, expires_at) WHERE deleted_at IS NULL AND status = 'active';

-- 复合索引引用列：避免额外的索引查找
CREATE INDEX IF NOT EXISTS idx_images_user_category_deleted ON images(user_id, category_id, deleted_at);
"#;

/// 初始化数据库架构
pub async fn init_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
    pool.execute(SCHEMA_SQL).await?;
    info!("Database schema initialized successfully");
    Ok(())
}

/// 创建默认管理员账号（使用 ON CONFLICT 避免竞态条件）
pub async fn create_default_admin(pool: &PgPool) -> Result<(String, String), anyhow::Error> {
    const ADMIN_USERNAME: &str = "admin";

    // 优先使用环境变量中的密码
    let admin_password = if let Ok(password) = std::env::var("ADMIN_PASSWORD") {
        if password.len() < 8 {
            warn!("ADMIN_PASSWORD is too short (min 8 characters), generating random password instead");
            generate_secure_password()
        } else {
            password
        }
    } else {
        // 没有设置环境变量，生成随机密码
        generate_secure_password()
    };

    let password_hash = AuthService::hash_password(&admin_password)?;
    let user_id = Uuid::new_v4();

    // 使用 INSERT ... ON CONFLICT DO NOTHING 避免竞态条件
    // 唯一约束在 username 字段上自动处理并发插入
    let result = sqlx::query(
        "INSERT INTO users (id, username, password_hash, role, created_at)
         VALUES ($1, $2, $3, 'admin', NOW())
         ON CONFLICT (username) DO NOTHING
         RETURNING id, username"
    )
    .bind(user_id)
    .bind(ADMIN_USERNAME)
    .bind(&password_hash)
    .fetch_optional(pool)
    .await;

    match result {
        Ok(Some(_)) => {
            info!("Default admin account created successfully");
            // 返回实际使用的密码（用于日志显示）
            Ok((ADMIN_USERNAME.to_string(), admin_password))
        }
        Ok(None) => {
            // 管理员已存在（由唯一约束触发）
            info!("Admin account already exists, skipping creation");
            Err(anyhow::anyhow!("Admin account already exists"))
        }
        Err(e) => {
            error!("Failed to create default admin account: {}", e);
            Err(anyhow::anyhow!("Failed to create default admin account: {}", e))
        }
    }
}

/// 生成安全的随机密码
fn generate_secure_password() -> String {
    use rand::{rngs::ThreadRng, RngExt};
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()-_=+";
    const PASSWORD_LENGTH: usize = 16;

    let mut rng = ThreadRng::default();
    let password: String = (0..PASSWORD_LENGTH)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    info!("Generated secure random password (length: {})", PASSWORD_LENGTH);
    password
}

/// 输出管理员账号信息到日志（用于启动时显示）
/// 注意：密码只在创建时返回一次，不会在这里记录，以避免泄露
pub async fn log_admin_credentials(pool: &PgPool) -> Result<(), anyhow::Error> {
    let result = sqlx::query_as::<_, (String,)>(
        "SELECT username FROM users WHERE role = 'admin' LIMIT 1"
    )
    .fetch_optional(pool)
    .await;

    match result {
        Ok(Some((username,))) => {
            info!("========================================");
            info!("        Admin Account");
            info!("========================================");
            info!("        Username: {}", username);
            info!("");
            info!("        ⚠️  If you haven't set ADMIN_PASSWORD environment variable,");
            info!("        the password was generated randomly on first startup.");
            info!("        Check your logs for the generated password shown at that time.");
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
