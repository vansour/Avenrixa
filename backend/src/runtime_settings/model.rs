use crate::config::Config;

pub const SETTING_SITE_NAME: &str = "site_name";
pub const SETTING_SITE_FAVICON_DATA_URL: &str = "site_favicon_data_url";
pub const SETTING_SYSTEM_INSTALLED: &str = "system_installed";
pub const SETTING_STORAGE_BACKEND: &str = "storage_backend";
pub const SETTING_LOCAL_STORAGE_PATH: &str = "local_storage_path";
pub const SETTING_MAIL_ENABLED: &str = "mail_enabled";
pub const SETTING_MAIL_SMTP_HOST: &str = "mail_smtp_host";
pub const SETTING_MAIL_SMTP_PORT: &str = "mail_smtp_port";
pub const SETTING_MAIL_SMTP_USER: &str = "mail_smtp_user";
pub const SETTING_MAIL_SMTP_PASSWORD: &str = "mail_smtp_password";
pub const SETTING_MAIL_FROM_EMAIL: &str = "mail_from_email";
pub const SETTING_MAIL_FROM_NAME: &str = "mail_from_name";
pub const SETTING_MAIL_LINK_BASE_URL: &str = "mail_link_base_url";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdminSettingPolicy {
    pub editable: bool,
    pub sensitive: bool,
    pub masked: bool,
    pub requires_confirmation: bool,
}

impl AdminSettingPolicy {
    const fn editable(requires_confirmation: bool) -> Self {
        Self {
            editable: true,
            sensitive: false,
            masked: false,
            requires_confirmation,
        }
    }

    const fn readonly() -> Self {
        Self {
            editable: false,
            sensitive: false,
            masked: false,
            requires_confirmation: false,
        }
    }
}

pub fn admin_setting_policy(key: &str) -> AdminSettingPolicy {
    match key {
        SETTING_SITE_NAME => AdminSettingPolicy::editable(false),
        SETTING_SYSTEM_INSTALLED => AdminSettingPolicy::readonly(),
        SETTING_STORAGE_BACKEND
        | SETTING_LOCAL_STORAGE_PATH
        | SETTING_MAIL_ENABLED
        | SETTING_MAIL_SMTP_HOST
        | SETTING_MAIL_SMTP_PORT
        | SETTING_MAIL_SMTP_USER
        | SETTING_MAIL_SMTP_PASSWORD
        | SETTING_MAIL_FROM_EMAIL
        | SETTING_MAIL_FROM_NAME
        | SETTING_MAIL_LINK_BASE_URL => AdminSettingPolicy::editable(true),
        SETTING_SITE_FAVICON_DATA_URL => AdminSettingPolicy {
            editable: false,
            sensitive: false,
            masked: true,
            requires_confirmation: false,
        },
        _ => AdminSettingPolicy::readonly(),
    }
}

pub fn mask_admin_setting_value(key: &str, value: &str) -> String {
    let policy = admin_setting_policy(key);
    if policy.masked && !value.trim().is_empty() {
        "********".to_string()
    } else {
        value.to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StorageBackend {
    Local,
}

impl StorageBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "local" => Some(Self::Local),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StorageSettingsSnapshot {
    pub storage_backend: StorageBackend,
    pub local_storage_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSettings {
    pub site_name: String,
    pub storage_backend: StorageBackend,
    pub local_storage_path: String,
    pub mail_enabled: bool,
    pub mail_smtp_host: String,
    pub mail_smtp_port: u16,
    pub mail_smtp_user: Option<String>,
    pub mail_smtp_password: Option<String>,
    pub mail_from_email: String,
    pub mail_from_name: String,
    pub mail_link_base_url: String,
}

impl RuntimeSettings {
    pub fn from_defaults(config: &Config) -> Self {
        let env_site_name = std::env::var("SITE_NAME")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let env_backend = std::env::var("STORAGE_BACKEND")
            .ok()
            .and_then(|v| StorageBackend::parse(v.trim()));

        Self {
            site_name: env_site_name.unwrap_or_default(),
            storage_backend: env_backend.unwrap_or(StorageBackend::Local),
            local_storage_path: config.storage.path.clone(),
            mail_enabled: config.mail.enabled,
            mail_smtp_host: config.mail.smtp_host.clone(),
            mail_smtp_port: config.mail.smtp_port,
            mail_smtp_user: config.mail.smtp_user.clone(),
            mail_smtp_password: config.mail.smtp_password.clone(),
            mail_from_email: config.mail.from_email.clone(),
            mail_from_name: config.mail.from_name.clone(),
            mail_link_base_url: config.mail.reset_link_base_url.clone(),
        }
    }

    pub fn storage_settings(&self) -> StorageSettingsSnapshot {
        StorageSettingsSnapshot {
            storage_backend: self.storage_backend,
            local_storage_path: self.local_storage_path.clone(),
        }
    }

    pub fn settings_version(&self) -> String {
        let payload = serde_json::json!({
            "site_name": self.site_name,
            "storage_backend": self.storage_backend.as_str(),
            "local_storage_path": self.local_storage_path,
            "mail_enabled": self.mail_enabled,
            "mail_smtp_host": self.mail_smtp_host,
            "mail_smtp_port": self.mail_smtp_port,
            "mail_smtp_user": self.mail_smtp_user,
            "mail_smtp_password": self.mail_smtp_password,
            "mail_from_email": self.mail_from_email,
            "mail_from_name": self.mail_from_name,
            "mail_link_base_url": self.mail_link_base_url,
        });
        blake3::hash(payload.to_string().as_bytes())
            .to_hex()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_backend_parse_is_strict() {
        assert_eq!(StorageBackend::parse("local"), Some(StorageBackend::Local));
        assert_eq!(StorageBackend::parse("unknown"), None);
    }

    #[test]
    fn settings_version_changes_when_runtime_settings_change() {
        let config = crate::config::Config::default();
        let mut settings = RuntimeSettings::from_defaults(&config);
        let original = settings.settings_version();
        settings.site_name = "Changed".to_string();
        assert_ne!(original, settings.settings_version());
    }
}
