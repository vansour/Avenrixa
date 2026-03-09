use super::types::{Config, ConfigError};
use lettre::Address;
use reqwest::Url;

impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.database.url.trim().is_empty() {
            return Err(ConfigError::DatabaseUrlEmpty);
        }
        if self.database.max_connections == 0 {
            return Err(ConfigError::InvalidPoolSize);
        }

        if self.redis.url.trim().is_empty() {
            return Err(ConfigError::RedisUrlEmpty);
        }
        if self.redis.ttl == 0 {
            return Err(ConfigError::InvalidTtl);
        }

        if self.storage.path.trim().is_empty() {
            return Err(ConfigError::StoragePathEmpty);
        }
        if self.storage.thumbnail_path.trim().is_empty() {
            return Err(ConfigError::ThumbnailPathEmpty);
        }
        if self.storage.allowed_extensions.is_empty() {
            return Err(ConfigError::AllowedExtensionsEmpty);
        }

        if self.server.max_upload_size == 0 {
            return Err(ConfigError::InvalidMaxUploadSize);
        }
        if self.server.rate_limit_per_second == 0 || self.server.rate_limit_burst == 0 {
            return Err(ConfigError::InvalidServerRateLimit);
        }

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
            return Err(ConfigError::InvalidDedupStrategy(
                self.image.dedup_strategy.clone(),
            ));
        }

        if self.cleanup.deleted_images_retention_days <= 0 {
            return Err(ConfigError::InvalidRetentionDays);
        }
        if self.cleanup.deleted_cleanup_interval_seconds == 0
            || self.cleanup.expiry_check_interval_seconds == 0
        {
            return Err(ConfigError::InvalidCleanupInterval);
        }

        match self.cookie.same_site.as_str() {
            "Strict" | "Lax" | "None" => {}
            _ => return Err(ConfigError::InvalidCookieSameSite),
        }
        if self.cookie.path.trim().is_empty() {
            return Err(ConfigError::InvalidCookiePath);
        }
        if self.cookie.max_age_seconds == 0 {
            return Err(ConfigError::InvalidCookieMaxAge);
        }

        if self.cache.list_ttl == 0 || self.cache.detail_ttl == 0 || self.cache.categories_ttl == 0
        {
            return Err(ConfigError::InvalidTtl);
        }

        if self.rate_limit.requests_per_minute == 0 || self.rate_limit.burst_size == 0 {
            return Err(ConfigError::InvalidTtl);
        }

        if self.mail.enabled {
            if self.mail.smtp_host.trim().is_empty() {
                return Err(ConfigError::MailSmtpHostEmpty);
            }
            if self.mail.smtp_port == 0 {
                return Err(ConfigError::InvalidMailSmtpPort);
            }
            if self.mail.from_email.trim().is_empty() {
                return Err(ConfigError::MailFromEmailEmpty);
            }
            if self.mail.from_email.parse::<Address>().is_err() {
                return Err(ConfigError::InvalidMailFromEmail(
                    self.mail.from_email.clone(),
                ));
            }
            if self.mail.reset_link_base_url.trim().is_empty() {
                return Err(ConfigError::MailResetLinkBaseUrlEmpty);
            }

            let reset_link = Url::parse(&self.mail.reset_link_base_url).map_err(|_| {
                ConfigError::InvalidResetLinkBaseUrl(self.mail.reset_link_base_url.clone())
            })?;
            if !matches!(reset_link.scheme(), "http" | "https") {
                return Err(ConfigError::InvalidResetLinkBaseUrl(
                    self.mail.reset_link_base_url.clone(),
                ));
            }

            let has_smtp_user = self
                .mail
                .smtp_user
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty());
            let has_smtp_password = self
                .mail
                .smtp_password
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty());
            if has_smtp_user != has_smtp_password {
                return Err(ConfigError::IncompleteSmtpCredentials);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_mail_config() -> Config {
        let mut config = Config::default();
        config.mail.enabled = true;
        config.mail.smtp_host = "smtp.example.com".to_string();
        config.mail.smtp_port = 587;
        config.mail.from_email = "noreply@example.com".to_string();
        config.mail.reset_link_base_url = "https://img.example.com/reset-password".to_string();
        config
    }

    #[test]
    fn validate_accepts_enabled_mail_with_complete_settings() {
        let config = valid_mail_config();

        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_mail_without_smtp_host() {
        let mut config = valid_mail_config();
        config.mail.smtp_host.clear();

        assert!(matches!(
            config.validate(),
            Err(ConfigError::MailSmtpHostEmpty)
        ));
    }

    #[test]
    fn validate_rejects_mail_with_partial_smtp_credentials() {
        let mut config = valid_mail_config();
        config.mail.smtp_user = Some("mailer".to_string());
        config.mail.smtp_password = None;

        assert!(matches!(
            config.validate(),
            Err(ConfigError::IncompleteSmtpCredentials)
        ));
    }

    #[test]
    fn validate_rejects_mail_with_invalid_reset_link() {
        let mut config = valid_mail_config();
        config.mail.reset_link_base_url = "mailto:reset@example.com".to_string();

        assert!(matches!(
            config.validate(),
            Err(ConfigError::InvalidResetLinkBaseUrl(_))
        ));
    }
}
