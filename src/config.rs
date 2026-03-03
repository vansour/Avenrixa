use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

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
                max_connections: 10,
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
        }
    }
}

impl Config {
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
        if let Ok(redis_url) = std::env::var("REDIS_URL") {
            config.redis.url = redis_url;
        }
        if let Ok(storage_path) = std::env::var("STORAGE_PATH") {
            config.storage.path = storage_path;
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

        config
    }

    pub fn addr(&self) -> SocketAddr {
        format!("{}:{}", self.server.host, self.server.port)
            .parse()
            .unwrap_or_else(|_| "0.0.0.0:8080".parse().expect("Invalid default address"))
    }
}
