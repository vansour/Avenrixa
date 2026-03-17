use crate::types::api::{
    AdminSettingsConfig, StorageBackendKind, TestS3StorageConfigRequest,
    UpdateAdminSettingsConfigRequest,
};
use dioxus::prelude::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum S3ProviderPreset {
    AwsS3,
    CloudflareR2,
    Minio,
    Other,
}

impl S3ProviderPreset {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AwsS3 => "aws-s3",
            Self::CloudflareR2 => "cloudflare-r2",
            Self::Minio => "minio",
            Self::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim() {
            "aws-s3" => Self::AwsS3,
            "cloudflare-r2" => Self::CloudflareR2,
            "minio" => Self::Minio,
            _ => Self::Other,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::AwsS3 => "AWS S3",
            Self::CloudflareR2 => "Cloudflare R2",
            Self::Minio => "MinIO",
            Self::Other => "其他兼容服务",
        }
    }

    pub fn endpoint_placeholder(self) -> &'static str {
        match self {
            Self::AwsS3 => "https://s3.us-east-1.amazonaws.com",
            Self::CloudflareR2 => "https://<ACCOUNT_ID>.r2.cloudflarestorage.com",
            Self::Minio => "http://127.0.0.1:9000",
            Self::Other => "https://s3.example.com",
        }
    }

    pub fn region_placeholder(self) -> &'static str {
        match self {
            Self::AwsS3 | Self::Minio | Self::Other => "us-east-1",
            Self::CloudflareR2 => "auto",
        }
    }

    pub fn endpoint_hint(self) -> &'static str {
        match self {
            Self::AwsS3 => "AWS S3 通常使用区域对应的服务地址，路径风格保持关闭。",
            Self::CloudflareR2 => {
                "R2 使用账户级 endpoint，region 建议填写 auto，路径风格保持关闭。"
            }
            Self::Minio => "MinIO 通常使用自建服务地址，并开启路径风格。",
            Self::Other => "填写兼容 S3 的 endpoint、region 与桶信息即可。",
        }
    }

    pub fn path_style_hint(self) -> &'static str {
        match self {
            Self::Minio => "使用路径风格（MinIO 通常需要开启）",
            Self::CloudflareR2 => "使用路径风格（R2 通常保持关闭）",
            Self::AwsS3 => "使用路径风格（AWS S3 通常保持关闭）",
            Self::Other => "使用路径风格（仅部分兼容服务需要开启）",
        }
    }

    pub fn default_region(self) -> Option<&'static str> {
        match self {
            Self::AwsS3 => Some("us-east-1"),
            Self::CloudflareR2 => Some("auto"),
            Self::Minio => Some("us-east-1"),
            Self::Other => None,
        }
    }

    pub fn default_endpoint(self) -> Option<&'static str> {
        match self {
            Self::AwsS3 => Some("https://s3.us-east-1.amazonaws.com"),
            Self::CloudflareR2 => None,
            Self::Minio => Some("http://127.0.0.1:9000"),
            Self::Other => None,
        }
    }

    pub fn default_force_path_style(self) -> Option<bool> {
        match self {
            Self::AwsS3 | Self::CloudflareR2 => Some(false),
            Self::Minio => Some(true),
            Self::Other => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct S3ProviderDraft {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub prefix: String,
    pub access_key: String,
    pub secret_key: String,
    pub secret_key_set: bool,
    pub force_path_style: bool,
}

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
    pub s3_provider_preset: Signal<S3ProviderPreset>,
    pub s3_provider_drafts: Signal<BTreeMap<S3ProviderPreset, S3ProviderDraft>>,
}

impl SettingsFormState {
    pub fn apply_loaded_config(&mut self, config: AdminSettingsConfig) {
        let provider_preset = infer_s3_provider_preset(
            config.s3_endpoint.as_deref(),
            config.s3_region.as_deref(),
            config.s3_force_path_style,
        );
        self.site_name.set(config.site_name);
        self.storage_backend.set(config.storage_backend);
        self.local_storage_path.set(config.local_storage_path);
        self.mail_enabled.set(config.mail_enabled);
        self.mail_smtp_host.set(config.mail_smtp_host);
        self.mail_smtp_port
            .set(display_mail_smtp_port(config.mail_smtp_port));
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
        self.s3_provider_preset.set(provider_preset);
        let mut drafts = BTreeMap::new();
        drafts.insert(provider_preset, self.current_s3_provider_draft());
        self.s3_provider_drafts.set(drafts);
    }

    pub fn is_s3_backend(&self) -> bool {
        (self.storage_backend)().is_s3()
    }

    pub fn switch_s3_provider_preset(mut self, preset: S3ProviderPreset) {
        let current_preset = (self.s3_provider_preset)();
        let mut drafts = (self.s3_provider_drafts)();
        drafts.insert(current_preset, self.current_s3_provider_draft());

        let next_draft = drafts
            .get(&preset)
            .cloned()
            .unwrap_or_else(|| self.provider_default_draft(preset));

        (self.s3_provider_preset).set(preset);
        self.apply_s3_provider_draft(&next_draft);
        drafts.insert(preset, next_draft);
        (self.s3_provider_drafts).set(drafts);
    }

    pub fn validate(&self) -> Result<(), String> {
        let site_name = (self.site_name)().trim().to_string();
        let local_storage_path = (self.local_storage_path)().trim().to_string();

        if site_name.is_empty() {
            return Err("网站名称不能为空".to_string());
        }

        match (self.storage_backend)() {
            StorageBackendKind::Unknown => {
                return Err("请选择存储后端".to_string());
            }
            StorageBackendKind::Local if local_storage_path.is_empty() => {
                return Err("本地存储路径不能为空".to_string());
            }
            _ => {}
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

        if self.is_s3_backend() {
            self.validate_s3_for_test()?;
        }

        Ok(())
    }

    pub fn validate_s3_for_test(&self) -> Result<(), String> {
        if !self.is_s3_backend() {
            return Err("当前未启用 S3 存储".to_string());
        }

        if !self.is_s3_configuration_complete() {
            return Err("S3 模式下请填写 endpoint/region/bucket/access_key/secret_key".to_string());
        }

        Ok(())
    }

    pub fn is_s3_configuration_complete(&self) -> bool {
        self.is_s3_backend()
            && !(self.s3_endpoint)().trim().is_empty()
            && !(self.s3_region)().trim().is_empty()
            && !(self.s3_bucket)().trim().is_empty()
            && !(self.s3_access_key)().trim().is_empty()
            && ((self.s3_secret_key_set)() || !(self.s3_secret_key)().trim().is_empty())
    }

    pub fn build_s3_test_request(&self) -> TestS3StorageConfigRequest {
        TestS3StorageConfigRequest {
            s3_endpoint: optional_trimmed((self.s3_endpoint)()),
            s3_region: optional_trimmed((self.s3_region)()),
            s3_bucket: optional_trimmed((self.s3_bucket)()),
            s3_prefix: optional_trimmed((self.s3_prefix)()),
            s3_access_key: optional_trimmed((self.s3_access_key)()),
            s3_secret_key: optional_trimmed((self.s3_secret_key)()),
            s3_secret_key_set: (self.s3_secret_key_set)(),
            s3_force_path_style: Some((self.s3_force_path_style)()),
        }
    }

    pub fn build_update_request(
        &self,
        expected_settings_version: Option<String>,
    ) -> UpdateAdminSettingsConfigRequest {
        UpdateAdminSettingsConfigRequest {
            expected_settings_version,
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

    fn current_s3_provider_draft(&self) -> S3ProviderDraft {
        S3ProviderDraft {
            endpoint: (self.s3_endpoint)(),
            region: (self.s3_region)(),
            bucket: (self.s3_bucket)(),
            prefix: (self.s3_prefix)(),
            access_key: (self.s3_access_key)(),
            secret_key: (self.s3_secret_key)(),
            secret_key_set: (self.s3_secret_key_set)(),
            force_path_style: (self.s3_force_path_style)(),
        }
    }

    fn provider_default_draft(&self, preset: S3ProviderPreset) -> S3ProviderDraft {
        let mut draft = self.current_s3_provider_draft();
        if let Some(endpoint) = preset.default_endpoint() {
            draft.endpoint = endpoint.to_string();
        } else if preset == S3ProviderPreset::CloudflareR2 {
            draft.endpoint.clear();
        }
        if let Some(region) = preset.default_region() {
            draft.region = region.to_string();
        }
        if let Some(force_path_style) = preset.default_force_path_style() {
            draft.force_path_style = force_path_style;
        }
        draft
    }

    fn apply_s3_provider_draft(&mut self, draft: &S3ProviderDraft) {
        self.s3_endpoint.set(draft.endpoint.clone());
        self.s3_region.set(draft.region.clone());
        self.s3_bucket.set(draft.bucket.clone());
        self.s3_prefix.set(draft.prefix.clone());
        self.s3_access_key.set(draft.access_key.clone());
        self.s3_secret_key.set(draft.secret_key.clone());
        self.s3_secret_key_set.set(draft.secret_key_set);
        self.s3_force_path_style.set(draft.force_path_style);
    }
}

pub fn default_mail_link_base_url(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_string()
}

pub(super) fn display_mail_smtp_port(port: u16) -> String {
    if port == 0 {
        String::new()
    } else {
        port.to_string()
    }
}

fn optional_trimmed(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn optional_u16(value: String) -> Option<u16> {
    value.trim().parse::<u16>().ok()
}

pub fn infer_s3_provider_preset(
    endpoint: Option<&str>,
    region: Option<&str>,
    force_path_style: bool,
) -> S3ProviderPreset {
    let endpoint = endpoint
        .map(str::trim)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let region = region
        .map(str::trim)
        .unwrap_or_default()
        .to_ascii_lowercase();

    if endpoint.contains(".r2.cloudflarestorage.com") || region == "auto" {
        S3ProviderPreset::CloudflareR2
    } else if endpoint.contains("amazonaws.com") {
        S3ProviderPreset::AwsS3
    } else if force_path_style && !endpoint.is_empty() {
        S3ProviderPreset::Minio
    } else {
        S3ProviderPreset::Other
    }
}

#[cfg(test)]
mod tests {
    use super::{S3ProviderPreset, infer_s3_provider_preset};

    #[test]
    fn infer_r2_provider_from_endpoint() {
        assert_eq!(
            infer_s3_provider_preset(
                Some("https://abc123.r2.cloudflarestorage.com"),
                Some("auto"),
                false
            ),
            S3ProviderPreset::CloudflareR2
        );
    }

    #[test]
    fn infer_aws_provider_from_endpoint() {
        assert_eq!(
            infer_s3_provider_preset(
                Some("https://s3.us-east-1.amazonaws.com"),
                Some("us-east-1"),
                false
            ),
            S3ProviderPreset::AwsS3
        );
    }

    #[test]
    fn infer_minio_provider_from_path_style() {
        assert_eq!(
            infer_s3_provider_preset(Some("http://127.0.0.1:9000"), Some("us-east-1"), true),
            S3ProviderPreset::Minio
        );
    }
}
