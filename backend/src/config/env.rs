use std::net::SocketAddr;

use super::types::{Config, DatabaseKind};

impl Config {
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(host) = std::env::var("SERVER_HOST") {
            config.server.host = host;
        }
        if let Ok(port) = std::env::var("SERVER_PORT") {
            config.server.port = port.parse().unwrap_or(8080);
        }
        if let Ok(rate_limit_per_second) = std::env::var("SERVER_RATE_LIMIT_PER_SECOND") {
            config.server.rate_limit_per_second = rate_limit_per_second.parse().unwrap_or(10);
        }
        if let Ok(rate_limit_burst) = std::env::var("SERVER_RATE_LIMIT_BURST") {
            config.server.rate_limit_burst = rate_limit_burst.parse().unwrap_or(30);
        }
        if let Ok(database_kind) = std::env::var("DATABASE_KIND")
            && let Ok(kind) = DatabaseKind::parse(&database_kind)
        {
            config.database.kind = kind;
        }
        if let Ok(db_url) = std::env::var("DATABASE_URL") {
            if let Some(kind) = DatabaseKind::infer_from_url(&db_url) {
                config.database.kind = kind;
            }
            config.database.url = db_url;
        }
        if let Ok(cache_url) = std::env::var("REDIS_URL") {
            config.cache_backend.url = if cache_url.trim().is_empty() {
                None
            } else {
                Some(cache_url)
            };
        }
        if let Ok(storage_path) = std::env::var("STORAGE_PATH") {
            config.storage.path = storage_path;
        }
        if let Ok(enable_file_check) = std::env::var("STORAGE_ENABLE_FILE_CHECK") {
            config.storage.enable_file_check = enable_file_check.parse().unwrap_or(true);
        }
        if let Ok(file_check_threshold) = std::env::var("STORAGE_FILE_CHECK_THRESHOLD") {
            config.storage.file_check_concurrent_threshold =
                file_check_threshold.parse().unwrap_or(50);
        }
        if let Ok(cleanup_enabled) = std::env::var("CLEANUP_ENABLED") {
            config.cleanup.enabled = cleanup_enabled.parse().unwrap_or(true);
        }
        if let Ok(expiry_interval) = std::env::var("CLEANUP_EXPIRY_CHECK_INTERVAL_SECONDS") {
            config.cleanup.expiry_check_interval_seconds = expiry_interval.parse().unwrap_or(3600);
        }
        if let Ok(cookie_secure) = std::env::var("AUTH_COOKIE_SECURE") {
            config.cookie.secure = cookie_secure.parse().unwrap_or(true);
        }
        if let Ok(cookie_same_site) = std::env::var("AUTH_COOKIE_SAME_SITE") {
            config.cookie.same_site = match cookie_same_site.to_ascii_lowercase().as_str() {
                "strict" => "Strict".to_string(),
                "lax" => "Lax".to_string(),
                "none" => "None".to_string(),
                _ => cookie_same_site,
            };
        }
        if let Ok(cookie_path) = std::env::var("AUTH_COOKIE_PATH") {
            config.cookie.path = cookie_path;
        }
        if let Ok(cookie_domain) = std::env::var("AUTH_COOKIE_DOMAIN") {
            config.cookie.domain = if cookie_domain.trim().is_empty() {
                None
            } else {
                Some(cookie_domain)
            };
        }
        if let Ok(cookie_max_age_seconds) = std::env::var("AUTH_COOKIE_MAX_AGE_SECONDS") {
            config.cookie.max_age_seconds = cookie_max_age_seconds.parse().unwrap_or(7 * 24 * 3600);
        }

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
            config.mail.smtp_user = if smtp_user.trim().is_empty() {
                None
            } else {
                Some(smtp_user)
            };
        }
        if let Ok(smtp_password) = std::env::var("SMTP_PASSWORD") {
            config.mail.smtp_password = if smtp_password.trim().is_empty() {
                None
            } else {
                Some(smtp_password)
            };
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
            && (dedup_strategy == "user" || dedup_strategy == "global")
        {
            config.image.dedup_strategy = dedup_strategy;
        }

        config
    }

    pub fn addr(&self) -> SocketAddr {
        format!("{}:{}", self.server.host, self.server.port)
            .parse()
            .unwrap_or_else(|_| "0.0.0.0:8080".parse().expect("Invalid default address"))
    }
}
