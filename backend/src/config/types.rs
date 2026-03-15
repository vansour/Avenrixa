use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 配置验证错误
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("数据库 URL 不能为空")]
    DatabaseUrlEmpty,
    #[error("不支持的数据库类型: {0}")]
    InvalidDatabaseKind(String),
    #[error("PostgreSQL 数据库 URL 必须以 postgresql:// 或 postgres:// 开头")]
    InvalidPostgresDatabaseUrl,
    #[error("MySQL / MariaDB 数据库 URL 必须以 mysql:// 或 mariadb:// 开头")]
    InvalidMySqlDatabaseUrl,
    #[error("SQLite 数据库地址必须是 sqlite: 前缀或文件路径")]
    InvalidSqliteDatabaseUrl,
    #[error("存储路径不能为空")]
    StoragePathEmpty,
    #[error("允许的扩展名列表不能为空")]
    AllowedExtensionsEmpty,
    #[error("数据库连接池大小必须大于 0")]
    InvalidPoolSize,
    #[error("最大上传大小必须大于 0")]
    InvalidMaxUploadSize,
    #[error("服务端限流配置必须大于 0")]
    InvalidServerRateLimit,
    #[error("无效的去重策略: {0}，必须是 'user' 或 'global'")]
    InvalidDedupStrategy(String),
    #[error("TTL 必须大于 0")]
    InvalidTtl,
    #[error("JPEG 质量必须在 1-100 之间")]
    InvalidJpegQuality,
    #[error("图片尺寸必须大于 0")]
    InvalidImageSize,
    #[error("Cookie SameSite 必须是 Strict/Lax/None")]
    InvalidCookieSameSite,
    #[error("Cookie Path 不能为空")]
    InvalidCookiePath,
    #[error("Cookie Max-Age 必须大于 0")]
    InvalidCookieMaxAge,
    #[error("清理间隔必须大于 0")]
    InvalidCleanupInterval,
    #[error("启用邮件服务时 SMTP_HOST 不能为空")]
    MailSmtpHostEmpty,
    #[error("启用邮件服务时 SMTP_PORT 必须大于 0")]
    InvalidMailSmtpPort,
    #[error("启用邮件服务时 MAIL_FROM 不能为空")]
    MailFromEmailEmpty,
    #[error("无效的 MAIL_FROM 邮箱地址: {0}")]
    InvalidMailFromEmail(String),
    #[error("启用邮件服务时 RESET_LINK_BASE_URL 不能为空")]
    MailResetLinkBaseUrlEmpty,
    #[error("无效的 RESET_LINK_BASE_URL: {0}")]
    InvalidResetLinkBaseUrl(String),
    #[error("SMTP_USER 和 SMTP_PASSWORD 必须同时配置或同时留空")]
    IncompleteSmtpCredentials,
}

pub(crate) fn default_max_connections() -> u32 {
    std::cmp::max(num_cpus::get_physical() * 4, 10) as u32
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    #[serde(alias = "redis")]
    pub cache_backend: CacheBackendConfig,
    pub storage: StorageConfig,
    #[serde(alias = "cache")]
    pub cache_policy: CachePolicyConfig,
    pub rate_limit: RateLimitConfig,
    pub cleanup: CleanupConfig,
    pub cookie: CookieConfig,
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
    #[serde(default = "default_frontend_dir")]
    pub frontend_dir: String,
}

pub(crate) fn default_frontend_dir() -> String {
    "/app/frontend/dist".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default)]
    pub kind: DatabaseKind,
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DatabaseKind {
    #[serde(rename = "postgresql")]
    #[default]
    Postgres,
    #[serde(rename = "mysql")]
    MySql,
    #[serde(rename = "sqlite")]
    Sqlite,
}

impl DatabaseKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Postgres => "postgresql",
            Self::MySql => "mysql",
            Self::Sqlite => "sqlite",
        }
    }

    pub fn parse(value: &str) -> Result<Self, ConfigError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "postgresql" | "postgres" => Ok(Self::Postgres),
            "mysql" => Ok(Self::MySql),
            "sqlite" => Ok(Self::Sqlite),
            other => Err(ConfigError::InvalidDatabaseKind(other.to_string())),
        }
    }

    pub fn infer_from_url(value: &str) -> Option<Self> {
        let trimmed = value.trim().to_ascii_lowercase();
        if matches!(
            trimmed.split(':').next(),
            Some("postgresql") | Some("postgres")
        ) {
            Some(Self::Postgres)
        } else if is_mysql_compatible_scheme(&trimmed) {
            Some(Self::MySql)
        } else if trimmed.starts_with("sqlite:") {
            Some(Self::Sqlite)
        } else {
            None
        }
    }
}

pub fn is_mysql_compatible_scheme(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().split(':').next(),
        Some("mysql") | Some("mariadb")
    )
}

pub fn normalize_mysql_compatible_url(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= "mariadb://".len()
        && trimmed[.."mariadb://".len()].eq_ignore_ascii_case("mariadb://")
    {
        format!("mysql://{}", &trimmed["mariadb://".len()..])
    } else {
        trimmed.to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheBackendConfig {
    pub url: Option<String>,
    pub key_prefix: String,
    pub ttl: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub path: String,
    pub allowed_extensions: Vec<String>,
    #[serde(default = "default_enable_file_check")]
    pub enable_file_check: bool,
    #[serde(default = "default_file_check_concurrent_threshold")]
    pub file_check_concurrent_threshold: usize,
}

pub(crate) fn default_enable_file_check() -> bool {
    true
}

pub(crate) fn default_file_check_concurrent_threshold() -> usize {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePolicyConfig {
    pub list_ttl: u64,
    pub detail_ttl: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupConfig {
    pub enabled: bool,
    pub expiry_check_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieConfig {
    pub secure: bool,
    pub same_site: String,
    pub path: String,
    pub domain: Option<String>,
    pub max_age_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    pub max_width: u32,
    pub max_height: u32,
    pub thumbnail_size: u32,
    pub jpeg_quality: u8,
    #[serde(default = "default_dedup_strategy")]
    pub dedup_strategy: String,
}

pub(crate) fn default_dedup_strategy() -> String {
    "user".to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, to_value};

    #[test]
    fn config_deserializes_legacy_cache_field_names() {
        let mut value = to_value(Config::default()).expect("default config should serialize");
        let object = value
            .as_object_mut()
            .expect("default config should serialize to object");

        let cache_backend = object
            .remove("cache_backend")
            .expect("cache_backend should exist");
        let cache_policy = object
            .remove("cache_policy")
            .expect("cache_policy should exist");
        object.insert("redis".to_string(), cache_backend);
        object.insert("cache".to_string(), cache_policy);

        let parsed: Config =
            serde_json::from_value(Value::Object(object.clone())).expect("legacy aliases work");

        assert_eq!(parsed.cache_backend.key_prefix, "img:");
        assert_eq!(parsed.cache_policy.list_ttl, 300);
    }
}
