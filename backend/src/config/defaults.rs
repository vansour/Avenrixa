use super::types::{
    CacheBackendConfig, CachePolicyConfig, CleanupConfig, Config, CookieConfig, DatabaseConfig,
    DatabaseKind, ImageConfig, MailConfig, RateLimitConfig, ServerConfig, StorageConfig,
    default_frontend_dir, default_max_connections,
};

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                max_upload_size: 50 * 1024 * 1024,
                rate_limit_per_second: 30,
                rate_limit_burst: 120,
                jwt_secret_min_length: 32,
                frontend_dir: default_frontend_dir(),
            },
            database: DatabaseConfig {
                kind: DatabaseKind::Postgres,
                url: String::new(),
                max_connections: default_max_connections(),
            },
            cache_backend: CacheBackendConfig {
                url: None,
                key_prefix: "img:".to_string(),
                ttl: 3600,
            },
            storage: StorageConfig {
                path: "/data/images".to_string(),
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
            cache_policy: CachePolicyConfig {
                list_ttl: 300,
                detail_ttl: 1800,
                categories_ttl: 3600,
            },
            rate_limit: RateLimitConfig {
                requests_per_minute: 100,
                burst_size: 30,
            },
            cleanup: CleanupConfig {
                enabled: true,
                deleted_images_retention_days: 30,
                deleted_cleanup_interval_seconds: 86400,
                expiry_check_interval_seconds: 3600,
            },
            cookie: CookieConfig {
                secure: true,
                same_site: "Strict".to_string(),
                path: "/".to_string(),
                domain: None,
                max_age_seconds: 7 * 24 * 3600,
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
        }
    }
}
