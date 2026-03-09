use crate::config::Config;
use crate::models::AdminSettingsConfig;

use super::validation::normalize_s3_prefix;

pub const SETTING_SITE_NAME: &str = "site_name";
pub const SETTING_STORAGE_BACKEND: &str = "storage_backend";
pub const SETTING_LOCAL_STORAGE_PATH: &str = "local_storage_path";
pub const SETTING_S3_ENDPOINT: &str = "s3_endpoint";
pub const SETTING_S3_REGION: &str = "s3_region";
pub const SETTING_S3_BUCKET: &str = "s3_bucket";
pub const SETTING_S3_PREFIX: &str = "s3_prefix";
pub const SETTING_S3_ACCESS_KEY: &str = "s3_access_key";
pub const SETTING_S3_SECRET_KEY: &str = "s3_secret_key";
pub const SETTING_S3_FORCE_PATH_STYLE: &str = "s3_force_path_style";

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

    const fn protected_sensitive() -> Self {
        Self {
            editable: false,
            sensitive: true,
            masked: true,
            requires_confirmation: false,
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
        SETTING_STORAGE_BACKEND
        | SETTING_LOCAL_STORAGE_PATH
        | SETTING_S3_ENDPOINT
        | SETTING_S3_REGION
        | SETTING_S3_BUCKET
        | SETTING_S3_PREFIX
        | SETTING_S3_FORCE_PATH_STYLE => AdminSettingPolicy::editable(true),
        SETTING_S3_ACCESS_KEY | SETTING_S3_SECRET_KEY => AdminSettingPolicy::protected_sensitive(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageBackend {
    Local,
    S3,
}

impl StorageBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::S3 => "s3",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("local") {
            Some(Self::Local)
        } else if value.eq_ignore_ascii_case("s3") {
            Some(Self::S3)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeSettings {
    pub site_name: String,
    pub storage_backend: StorageBackend,
    pub local_storage_path: String,
    pub s3_endpoint: Option<String>,
    pub s3_region: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_prefix: Option<String>,
    pub s3_access_key: Option<String>,
    pub s3_secret_key: Option<String>,
    pub s3_force_path_style: bool,
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
        let env_s3_endpoint = std::env::var("S3_ENDPOINT")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let env_s3_region = std::env::var("S3_REGION")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let env_s3_bucket = std::env::var("S3_BUCKET")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let env_s3_prefix = std::env::var("S3_PREFIX")
            .ok()
            .map(|v| normalize_s3_prefix(v.trim()));
        let env_s3_access_key = std::env::var("S3_ACCESS_KEY")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let env_s3_secret_key = std::env::var("S3_SECRET_KEY")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let env_s3_force_path_style = std::env::var("S3_FORCE_PATH_STYLE")
            .ok()
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(true);

        Self {
            site_name: env_site_name.unwrap_or_else(|| "Vansour Image".to_string()),
            storage_backend: env_backend.unwrap_or(StorageBackend::Local),
            local_storage_path: config.storage.path.clone(),
            s3_endpoint: env_s3_endpoint,
            s3_region: env_s3_region,
            s3_bucket: env_s3_bucket,
            s3_prefix: env_s3_prefix,
            s3_access_key: env_s3_access_key,
            s3_secret_key: env_s3_secret_key,
            s3_force_path_style: env_s3_force_path_style,
        }
    }

    pub fn to_admin_config(&self) -> AdminSettingsConfig {
        AdminSettingsConfig {
            site_name: self.site_name.clone(),
            storage_backend: self.storage_backend.as_str().to_string(),
            local_storage_path: self.local_storage_path.clone(),
            s3_endpoint: self.s3_endpoint.clone(),
            s3_region: self.s3_region.clone(),
            s3_bucket: self.s3_bucket.clone(),
            s3_prefix: self.s3_prefix.clone(),
            s3_access_key: self.s3_access_key.clone(),
            s3_secret_key_set: self
                .s3_secret_key
                .as_ref()
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false),
            s3_force_path_style: self.s3_force_path_style,
            restart_required: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_backend_parse_is_strict() {
        assert_eq!(StorageBackend::parse("local"), Some(StorageBackend::Local));
        assert_eq!(StorageBackend::parse("s3"), Some(StorageBackend::S3));
        assert_eq!(StorageBackend::parse("LOCAL"), Some(StorageBackend::Local));
        assert_eq!(StorageBackend::parse("unknown"), None);
    }

    #[test]
    fn admin_config_marks_runtime_settings_as_hot_reloadable() {
        let config = crate::config::Config::default();
        let settings = RuntimeSettings::from_defaults(&config);

        assert!(!settings.to_admin_config().restart_required);
    }
}
