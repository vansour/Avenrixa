use crate::types::api::{
    AdminSettingsConfig, StorageBackendKind, UpdateAdminSettingsConfigRequest,
};
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub struct SettingsFormState {
    pub site_name: Signal<String>,
    pub storage_backend: Signal<StorageBackendKind>,
    pub local_storage_path: Signal<String>,
    pub mail_enabled: Signal<bool>,
    pub mail_smtp_host: Signal<String>,
    pub mail_smtp_port: Signal<String>,
    pub mail_smtp_user: Signal<String>,
    pub mail_smtp_password: Signal<String>,
    pub mail_smtp_password_set: Signal<bool>,
    pub mail_from_email: Signal<String>,
    pub mail_from_name: Signal<String>,
    pub mail_link_base_url: Signal<String>,
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
        self.mail_enabled.set(config.mail_enabled);
        self.mail_smtp_host.set(config.mail_smtp_host);
        self.mail_smtp_port.set(config.mail_smtp_port.to_string());
        self.mail_smtp_user
            .set(config.mail_smtp_user.unwrap_or_default());
        self.mail_smtp_password.set(String::new());
        self.mail_smtp_password_set
            .set(config.mail_smtp_password_set);
        self.mail_from_email.set(config.mail_from_email);
        self.mail_from_name.set(config.mail_from_name);
        self.mail_link_base_url
            .set(default_mail_link_base_url(config.mail_link_base_url));
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
        (self.storage_backend)().is_s3()
    }

    pub fn validate(&self) -> Result<(), String> {
        let site_name = (self.site_name)().trim().to_string();
        let local_storage_path = (self.local_storage_path)().trim().to_string();

        if site_name.is_empty() || local_storage_path.is_empty() {
            return Err("网站名称和本地存储路径不能为空".to_string());
        }

        if (self.mail_enabled)() {
            let mail_smtp_host = (self.mail_smtp_host)().trim().to_string();
            let mail_smtp_port = (self.mail_smtp_port)().trim().to_string();
            let mail_smtp_user = (self.mail_smtp_user)().trim().to_string();
            let mail_smtp_password = (self.mail_smtp_password)().trim().to_string();
            let mail_from_email = (self.mail_from_email)().trim().to_string();
            let mail_link_base_url = (self.mail_link_base_url)().trim().to_string();

            if mail_smtp_host.is_empty()
                || mail_from_email.is_empty()
                || mail_link_base_url.is_empty()
            {
                return Err("启用邮件服务后请填写 SMTP 主机、发件邮箱和站点访问地址".to_string());
            }

            if mail_smtp_port
                .parse::<u16>()
                .ok()
                .filter(|port| *port > 0)
                .is_none()
            {
                return Err("SMTP 端口必须是大于 0 的整数".to_string());
            }

            let has_smtp_user = !mail_smtp_user.is_empty();
            let has_smtp_password = !mail_smtp_password.is_empty()
                || ((self.mail_smtp_password_set)() && has_smtp_user);
            if has_smtp_user != has_smtp_password {
                return Err("SMTP 用户名和密码必须同时配置，或同时留空".to_string());
            }
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
            mail_enabled: (self.mail_enabled)(),
            mail_smtp_host: (self.mail_smtp_host)().trim().to_string(),
            mail_smtp_port: optional_u16((self.mail_smtp_port)()),
            mail_smtp_user: optional_trimmed((self.mail_smtp_user)()),
            mail_smtp_password: optional_trimmed((self.mail_smtp_password)()),
            mail_from_email: (self.mail_from_email)().trim().to_string(),
            mail_from_name: (self.mail_from_name)().trim().to_string(),
            mail_link_base_url: (self.mail_link_base_url)().trim().to_string(),
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

pub fn default_mail_link_base_url(value: impl AsRef<str>) -> String {
    let value = value.as_ref().trim();
    if !value.is_empty() {
        return value.to_string();
    }

    web_sys::window()
        .and_then(|window| window.location().origin().ok())
        .unwrap_or_default()
}

fn optional_trimmed(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn optional_u16(value: String) -> Option<u16> {
    value.trim().parse::<u16>().ok()
}
