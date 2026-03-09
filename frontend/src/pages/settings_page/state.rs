use crate::types::api::{AdminSettingsConfig, UpdateAdminSettingsConfigRequest};
use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub struct SettingsFormState {
    pub site_name: Signal<String>,
    pub storage_backend: Signal<String>,
    pub local_storage_path: Signal<String>,
    pub s3_endpoint: Signal<String>,
    pub s3_region: Signal<String>,
    pub s3_bucket: Signal<String>,
    pub s3_prefix: Signal<String>,
    pub s3_access_key: Signal<String>,
    pub s3_secret_key: Signal<String>,
    pub s3_secret_key_set: Signal<bool>,
    pub s3_force_path_style: Signal<bool>,
}

impl SettingsFormState {
    pub fn apply_loaded_config(&mut self, config: AdminSettingsConfig) {
        self.site_name.set(config.site_name);
        self.storage_backend.set(config.storage_backend);
        self.local_storage_path.set(config.local_storage_path);
        self.s3_endpoint.set(config.s3_endpoint.unwrap_or_default());
        self.s3_region.set(config.s3_region.unwrap_or_default());
        self.s3_bucket.set(config.s3_bucket.unwrap_or_default());
        self.s3_prefix.set(config.s3_prefix.unwrap_or_default());
        self.s3_access_key
            .set(config.s3_access_key.unwrap_or_default());
        self.s3_secret_key.set(String::new());
        self.s3_secret_key_set.set(config.s3_secret_key_set);
        self.s3_force_path_style.set(config.s3_force_path_style);
    }

    pub fn is_s3_backend(&self) -> bool {
        (self.storage_backend)() == "s3"
    }

    pub fn validate(&self) -> Result<(), String> {
        let site_name = (self.site_name)().trim().to_string();
        let local_storage_path = (self.local_storage_path)().trim().to_string();

        if site_name.is_empty() || local_storage_path.is_empty() {
            return Err("网站名称和本地存储路径不能为空".to_string());
        }

        if self.is_s3_backend()
            && ((self.s3_endpoint)().trim().is_empty()
                || (self.s3_region)().trim().is_empty()
                || (self.s3_bucket)().trim().is_empty()
                || (self.s3_access_key)().trim().is_empty()
                || (!(self.s3_secret_key_set)() && (self.s3_secret_key)().trim().is_empty()))
        {
            return Err("S3 模式下请填写 endpoint/region/bucket/access_key/secret_key".to_string());
        }

        Ok(())
    }

    pub fn build_update_request(&self) -> UpdateAdminSettingsConfigRequest {
        UpdateAdminSettingsConfigRequest {
            site_name: (self.site_name)().trim().to_string(),
            storage_backend: (self.storage_backend)(),
            local_storage_path: (self.local_storage_path)().trim().to_string(),
            s3_endpoint: optional_trimmed((self.s3_endpoint)()),
            s3_region: optional_trimmed((self.s3_region)()),
            s3_bucket: optional_trimmed((self.s3_bucket)()),
            s3_prefix: optional_trimmed((self.s3_prefix)()),
            s3_access_key: optional_trimmed((self.s3_access_key)()),
            s3_secret_key: optional_trimmed((self.s3_secret_key)()),
            s3_force_path_style: Some((self.s3_force_path_style)()),
        }
    }
}

fn optional_trimmed(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}
