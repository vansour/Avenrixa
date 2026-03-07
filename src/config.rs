use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use thiserror::Error;

/// 配置验证错误
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("数据库 URL 不能为空")]
    DatabaseUrlEmpty,
    #[error("Redis URL 不能为空")]
    RedisUrlEmpty,
    #[error("存储路径不能为空")]
    StoragePathEmpty,
    #[error("缩略图路径不能为空")]
    ThumbnailPathEmpty,
    #[error("允许的扩展名列表不能为空")]
    AllowedExtensionsEmpty,
    #[error("数据库连接池大小必须大于 0")]
    InvalidPoolSize,
    #[error("JWT secret 最小长度: {min}, 实际: {actual}")]
    JwtSecretTooShort { min: usize, actual: usize },
    #[error("最大上传大小必须大于 0")]
    InvalidMaxUploadSize,
    #[error("无效的去重策略: {0}，必须是 'user' 或 'global'")]
    InvalidDedupStrategy(String),
    #[error("无效的文件检查并发阈值: {0}")]
    InvalidFileCheckThreshold(String),
    #[error("保留图片天数必须大于 0")]
    InvalidRetentionDays,
    #[error("TTL 必须大于 0")]
    InvalidTtl,
    #[error("JPEG 质量必须在 1-100 之间")]
    InvalidJpegQuality,
    #[error("图片尺寸必须大于 0")]
    InvalidImageSize,
}

// 获取默认数据库连接池大小
// 对于 I/O 密集型应用（图片处理 + 文件 I/O），使用更高的连接数
fn default_max_connections() -> u32 {
    // 基础连接：每个物理核心 4 个（用于并发查询）
    // 额外连接：至少 10 个（用于处理突发请求和后台任务）
    std::cmp::max(num_cpus::get_physical() * 4, 10) as u32
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub storage: StorageConfig,
    pub cache: CacheConfig,
    pub rate_limit: RateLimitConfig,
    pub cleanup: CleanupConfig,
    pub mail: MailConfig,
    pub image: ImageConfig,
    pub admin: AdminConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_upload_size: usize,
    pub rate_limit_per_second: u32,
    pub rate_limit_burst: u32,
    pub jwt_secret_min_length: usize,
    #[serde(default = "default_cors_origins")]
    pub cors_origins: String,
}

fn default_cors_origins() -> String {
    "*".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub key_prefix: String,
    pub ttl: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub path: String,
    pub thumbnail_path: String,
    pub allowed_extensions: Vec<String>,
    /// 是否在查询时检查文件存在性（默认 true）
    #[serde(default = "default_enable_file_check")]
    pub enable_file_check: bool,
    /// 检查文件存在时的并发阈值（超过此数量时使用并发检查）
    #[serde(default = "default_file_check_concurrent_threshold")]
    pub file_check_concurrent_threshold: usize,
}

fn default_enable_file_check() -> bool {
    true
}

fn default_file_check_concurrent_threshold() -> usize {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub list_ttl: u64,
    pub detail_ttl: u64,
    pub categories_ttl: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupConfig {
    pub deleted_images_retention_days: i64,
    pub expiry_check_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    pub max_width: u32,
    pub max_height: u32,
    pub thumbnail_size: u32,
    pub jpeg_quality: u8,
    /// 去重策略：'user' 表示仅在同一用户内去重，'global' 表示全局去重
    #[serde(default = "default_dedup_strategy")]
    pub dedup_strategy: String,
}

fn default_dedup_strategy() -> String {
    "user".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    pub username: String,
    pub password: String,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            username: "username".to_string(),
            password: "password".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailConfig {
    pub enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: Option<String>,
    pub smtp_password: Option<String>,
    pub from_email: String,
    pub from_name: String,
    pub reset_link_base_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                max_upload_size: 50 * 1024 * 1024,
                rate_limit_per_second: 10,
                rate_limit_burst: 30,
                jwt_secret_min_length: 32,
                cors_origins: "*".to_string(),
            },
            database: DatabaseConfig {
                url: "postgresql://user:pass@postgres:5432/image".to_string(),
                max_connections: default_max_connections(),
            },
            redis: RedisConfig {
                url: "redis://dragonfly:6379".to_string(),
                key_prefix: "img:".to_string(),
                ttl: 3600,
            },
            storage: StorageConfig {
                path: "/data/images".to_string(),
                thumbnail_path: "/data/thumbnails".to_string(),
                allowed_extensions: vec![
                    "jpg".to_string(),
                    "jpeg".to_string(),
                    "png".to_string(),
                    "gif".to_string(),
                    "webp".to_string(),
                    "svg".to_string(),
                ],
                enable_file_check: true,
                file_check_concurrent_threshold: 50,
            },
            cache: CacheConfig {
                list_ttl: 300,           // 5分钟
                detail_ttl: 1800,        // 30分钟
                categories_ttl: 3600,     // 1小时
            },
            rate_limit: RateLimitConfig {
                requests_per_minute: 100,
                burst_size: 30,
            },
            cleanup: CleanupConfig {
                deleted_images_retention_days: 30,
                expiry_check_interval_seconds: 3600,
            },
            image: ImageConfig {
                max_width: 1920,
                max_height: 1080,
                thumbnail_size: 300,
                jpeg_quality: 85,
                dedup_strategy: "user".to_string(),
            },
            mail: MailConfig {
                enabled: false,
                smtp_host: "localhost".to_string(),
                smtp_port: 587,
                smtp_user: None,
                smtp_password: None,
                from_email: "noreply@example.com".to_string(),
                from_name: "Vansour Image".to_string(),
                reset_link_base_url: "http://localhost:8080/reset-password".to_string(),
            },
            admin: AdminConfig {
                username: "username".to_string(),
                password: "password".to_string(),
            },
        }
    }
}

impl Config {
    /// 验证配置是否有效
    pub fn validate(&self) -> Result<(), ConfigError> {
        // 验证数据库配置
        if self.database.url.trim().is_empty() {
            return Err(ConfigError::DatabaseUrlEmpty);
        }
        if self.database.max_connections == 0 {
            return Err(ConfigError::InvalidPoolSize);
        }

        // 验证 Redis 配置
        if self.redis.url.trim().is_empty() {
            return Err(ConfigError::RedisUrlEmpty);
        }
        if self.redis.ttl == 0 {
            return Err(ConfigError::InvalidTtl);
        }

        // 验证存储配置
        if self.storage.path.trim().is_empty() {
            return Err(ConfigError::StoragePathEmpty);
        }
        if self.storage.thumbnail_path.trim().is_empty() {
            return Err(ConfigError::ThumbnailPathEmpty);
        }
        if self.storage.allowed_extensions.is_empty() {
            return Err(ConfigError::AllowedExtensionsEmpty);
        }

        // 验证服务器配置
        if self.server.max_upload_size == 0 {
            return Err(ConfigError::InvalidMaxUploadSize);
        }

        // 验证图片配置
        if self.image.max_width == 0 || self.image.max_height == 0 {
            return Err(ConfigError::InvalidImageSize);
        }
        if self.image.thumbnail_size == 0 {
            return Err(ConfigError::InvalidImageSize);
        }
        if !(1..=100).contains(&self.image.jpeg_quality) {
            return Err(ConfigError::InvalidJpegQuality);
        }
        if self.image.dedup_strategy != "user" && self.image.dedup_strategy != "global" {
            return Err(ConfigError::InvalidDedupStrategy(self.image.dedup_strategy.clone()));
        }

        // 验证清理配置
        if self.cleanup.deleted_images_retention_days <= 0 {
            return Err(ConfigError::InvalidRetentionDays);
        }
        if self.cleanup.expiry_check_interval_seconds == 0 {
            return Err(ConfigError::InvalidTtl);
        }

        // 验证缓存配置
        if self.cache.list_ttl == 0 || self.cache.detail_ttl == 0 || self.cache.categories_ttl == 0 {
            return Err(ConfigError::InvalidTtl);
        }

        // 验证限流配置
        if self.rate_limit.requests_per_minute == 0 || self.rate_limit.burst_size == 0 {
            return Err(ConfigError::InvalidTtl);
        }

        Ok(())
    }

    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(host) = std::env::var("SERVER_HOST") {
            config.server.host = host;
        }
        if let Ok(port) = std::env::var("SERVER_PORT") {
            config.server.port = port.parse().unwrap_or(8080);
        }
        if let Ok(cors_origins) = std::env::var("CORS_ORIGINS") {
            config.server.cors_origins = cors_origins;
        }
        if let Ok(db_url) = std::env::var("DATABASE_URL") {
            config.database.url = db_url;
        }
        if let Ok(max_connections) = std::env::var("DATABASE_MAX_CONNECTIONS") {
            config.database.max_connections = max_connections.parse().unwrap_or(default_max_connections());
        }
        if let Ok(redis_url) = std::env::var("REDIS_URL") {
            config.redis.url = redis_url;
        }
        if let Ok(storage_path) = std::env::var("STORAGE_PATH") {
            config.storage.path = storage_path;
        }
        if let Ok(enable_file_check) = std::env::var("STORAGE_ENABLE_FILE_CHECK") {
            config.storage.enable_file_check = enable_file_check.parse().unwrap_or(true);
        }
        if let Ok(file_check_threshold) = std::env::var("STORAGE_FILE_CHECK_THRESHOLD") {
            config.storage.file_check_concurrent_threshold = file_check_threshold.parse().unwrap_or(50);
        }

        // 邮件配置
        if let Ok(mail_enabled) = std::env::var("MAIL_ENABLED") {
            config.mail.enabled = mail_enabled.parse().unwrap_or(false);
        }
        if let Ok(smtp_host) = std::env::var("SMTP_HOST") {
            config.mail.smtp_host = smtp_host;
        }
        if let Ok(smtp_port) = std::env::var("SMTP_PORT") {
            config.mail.smtp_port = smtp_port.parse().unwrap_or(587);
        }
        if let Ok(smtp_user) = std::env::var("SMTP_USER") {
            config.mail.smtp_user = Some(smtp_user);
        }
        if let Ok(smtp_password) = std::env::var("SMTP_PASSWORD") {
            config.mail.smtp_password = Some(smtp_password);
        }
        if let Ok(from_email) = std::env::var("MAIL_FROM") {
            config.mail.from_email = from_email;
        }
        if let Ok(from_name) = std::env::var("MAIL_FROM_NAME") {
            config.mail.from_name = from_name;
        }
        if let Ok(reset_link_base_url) = std::env::var("RESET_LINK_BASE_URL") {
            config.mail.reset_link_base_url = reset_link_base_url;
        }

        // 图片处理配置
        if let Ok(max_width) = std::env::var("IMAGE_MAX_WIDTH") {
            config.image.max_width = max_width.parse().unwrap_or(1920);
        }
        if let Ok(max_height) = std::env::var("IMAGE_MAX_HEIGHT") {
            config.image.max_height = max_height.parse().unwrap_or(1080);
        }
        if let Ok(thumbnail_size) = std::env::var("IMAGE_THUMBNAIL_SIZE") {
            config.image.thumbnail_size = thumbnail_size.parse().unwrap_or(300);
        }
        if let Ok(jpeg_quality) = std::env::var("IMAGE_JPEG_QUALITY") {
            config.image.jpeg_quality = jpeg_quality.parse().unwrap_or(85);
        }
        if let Ok(dedup_strategy) = std::env::var("IMAGE_DEDUP_STRATEGY")
            && (dedup_strategy == "user" || dedup_strategy == "global") {
            config.image.dedup_strategy = dedup_strategy;
        }

        // 管理员账户配置
        if let Ok(admin_username) = std::env::var("ADMIN_USERNAME") {
            config.admin.username = admin_username;
        }
        if let Ok(admin_password) = std::env::var("ADMIN_PASSWORD") {
            config.admin.password = admin_password;
        }

        config
    }

    pub fn addr(&self) -> SocketAddr {
        format!("{}:{}", self.server.host, self.server.port)
            .parse()
            .unwrap_or_else(|_| "0.0.0.0:8080".parse().expect("Invalid default address"))
    }
}
