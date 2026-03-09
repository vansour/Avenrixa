use crate::auth::AuthService;
use crate::cache::Cache;
use crate::config::Config;
use crate::domain::admin::AdminDomainService;
use crate::domain::auth::DefaultAuthDomainService;
use crate::domain::image::ImageDomainService;
use crate::domain::image::repository::{PostgresCategoryRepository, PostgresImageRepository};
use crate::file_queue::FileSaveQueue;
use crate::image_processor::ImageProcessor;
use crate::runtime_settings::RuntimeSettingsService;
use crate::storage_backend::StorageManager;
use redis::aio::ConnectionManager;
use sqlx::{Executor, PgPool, Row};
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub redis: ConnectionManager,
    pub config: Config,
    pub auth: AuthService,
    pub auth_domain_service: Option<Arc<DefaultAuthDomainService>>,
    pub image_domain_service:
        Option<Arc<ImageDomainService<PostgresImageRepository, PostgresCategoryRepository>>>,
    pub admin_domain_service: Option<Arc<AdminDomainService>>,
    pub runtime_settings: Arc<RuntimeSettingsService>,
    pub storage_manager: Arc<StorageManager>,
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

-- 分类表
CREATE TABLE IF NOT EXISTS categories (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, name)
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
CREATE INDEX IF NOT EXISTS idx_categories_user_id ON categories(user_id);
CREATE INDEX IF NOT EXISTS idx_categories_created_at ON categories(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_image_tags_image_id ON image_tags(image_id);
CREATE INDEX IF NOT EXISTS idx_image_tags_tag ON image_tags(tag);
CREATE INDEX IF NOT EXISTS idx_image_tags_image_tag ON image_tags(image_id, tag);
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
CREATE INDEX IF NOT EXISTS idx_image_tags_tag_trgm ON image_tags USING gin (tag gin_trgm_ops);

-- 分页索引优化
CREATE INDEX IF NOT EXISTS idx_images_user_status_partial ON images(user_id, status) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_images_user_category_status_partial ON images(user_id, category_id, status) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_images_user_status_created_partial ON images(user_id, status, created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_images_user_status_expires_partial ON images(user_id, status, expires_at) WHERE deleted_at IS NULL AND status = 'active';
CREATE INDEX IF NOT EXISTS idx_images_user_deleted_at_partial ON images(user_id, deleted_at DESC) WHERE deleted_at IS NOT NULL;

-- 复合索引引用列
CREATE INDEX IF NOT EXISTS idx_images_user_category_deleted ON images(user_id, category_id, deleted_at);
"#;

/// 唯一文件名约束（用于避免重复复制导致的物理文件覆盖）
const UNIQUE_FILENAME_INDEX_SQL: &str =
    "CREATE UNIQUE INDEX IF NOT EXISTS uq_images_filename ON images(filename);";

/// 初始化数据库架构
pub async fn init_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
    pool.execute(SCHEMA_SQL).await?;
    if let Err(e) = sqlx::query(UNIQUE_FILENAME_INDEX_SQL).execute(pool).await {
        warn!(
            "Failed to create unique filename index (existing duplicate rows may exist): {}",
            e
        );
    }
    info!("Database schema initialized successfully");
    Ok(())
}

/// 默认管理员用户名
pub const DEFAULT_ADMIN_USERNAME: &str = "username";

/// 默认管理员密码
pub const DEFAULT_ADMIN_PASSWORD: &str = "password";

/// 管理员账户初始化结果
pub struct AdminAccountInit {
    pub username: String,
    pub created: bool,
    pub using_default_password: bool,
}

/// 创建管理员账户（仅首次）
///
/// 如果管理员已存在，则不更新密码
pub async fn create_admin_account(pool: &PgPool) -> Result<AdminAccountInit, anyhow::Error> {
    let admin_username =
        std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| DEFAULT_ADMIN_USERNAME.to_string());
    let admin_password =
        std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| DEFAULT_ADMIN_PASSWORD.to_string());
    let using_default_password = admin_password == DEFAULT_ADMIN_PASSWORD;
    let password_hash = AuthService::hash_password(&admin_password)?;

    // 使用 INSERT ... ON CONFLICT DO NOTHING 来创建管理员（仅首次）
    // 如果用户已存在，则不会更新密码
    let result = sqlx::query(
        "INSERT INTO users (id, username, password_hash, role, created_at)
         VALUES ($1, $2, $3, 'admin', NOW())
         ON CONFLICT (id) DO NOTHING
         RETURNING username",
    )
    .bind(ADMIN_USER_ID)
    .bind(&admin_username)
    .bind(&password_hash)
    .fetch_optional(pool)
    .await;

    match result {
        Ok(Some(row)) => {
            let username: String = row.get("username");
            info!("Admin account created: {}", username);
            if using_default_password {
                warn!(
                    "Admin account initialized with default password. Set ADMIN_PASSWORD to a strong secret."
                );
            } else {
                info!("Admin password loaded from ADMIN_PASSWORD environment variable");
            }

            Ok(AdminAccountInit {
                username,
                created: true,
                using_default_password,
            })
        }
        Ok(None) => {
            // 用户已存在，使用现有账户
            info!("Admin account already exists");
            let existing = sqlx::query("SELECT username FROM users WHERE id = $1")
                .bind(ADMIN_USER_ID)
                .fetch_optional(pool)
                .await?;

            match existing {
                Some(row) => {
                    let username: String = row.get("username");
                    info!("Using existing admin account: {}", username);
                    Ok(AdminAccountInit {
                        username,
                        created: false,
                        using_default_password,
                    })
                }
                None => {
                    // 不应该发生，但返回错误
                    error!("Admin account not found in database");
                    Err(anyhow::anyhow!("Admin account not found"))
                }
            }
        }
        Err(e) => {
            error!("Failed to create admin account: {}", e);
            Err(anyhow::anyhow!("Failed to create admin account: {}", e))
        }
    }
}

impl AppState {
    /// 清除用户图片缓存
    pub async fn invalidate_user_image_cache(&self, user_id: Uuid) -> Result<(), anyhow::Error> {
        let mut redis = self.redis.clone();
        Cache::del_pattern(
            &mut redis,
            &crate::cache::ImageCache::images_invalidate(user_id),
        )
        .await
    }

    /// 清除用户分类缓存
    pub async fn invalidate_user_category_cache(&self, user_id: Uuid) -> Result<(), anyhow::Error> {
        let mut redis = self.redis.clone();
        Cache::del_pattern(
            &mut redis,
            &crate::cache::ImageCache::categories_invalidate(user_id),
        )
        .await
    }

    /// 清除用户所有缓存
    pub async fn invalidate_user_caches(&self, user_id: Uuid) -> Result<(), anyhow::Error> {
        self.invalidate_user_image_cache(user_id).await?;
        self.invalidate_user_category_cache(user_id).await?;
        Ok(())
    }
}
