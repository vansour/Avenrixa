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
}

impl SettingsFormState {
    pub fn apply_loaded_config(&mut self, config: AdminSettingsConfig) {
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

        Ok(())
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
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::{ScopeId, Signal, VirtualDom, rsx};

    struct TestSettingsFormHarness {
        _dom: VirtualDom,
        form: SettingsFormState,
    }

    impl TestSettingsFormHarness {
        fn new(form: impl FnOnce() -> SettingsFormState) -> Self {
            let dom = VirtualDom::new(|| rsx! {});
            let form = dom.in_scope(ScopeId::ROOT, form);
            Self { _dom: dom, form }
        }
    }

    fn sample_form_harness() -> TestSettingsFormHarness {
        TestSettingsFormHarness::new(|| SettingsFormState {
            site_name: Signal::new("  Avenrixa Console  ".to_string()),
            storage_backend: Signal::new(StorageBackendKind::Local),
            local_storage_path: Signal::new(" /data/images ".to_string()),
            mail_enabled: Signal::new(true),
            mail_smtp_host: Signal::new(" smtp.example.com ".to_string()),
            mail_smtp_port: Signal::new(" 587 ".to_string()),
            mail_smtp_user: Signal::new(" mailer ".to_string()),
            mail_smtp_password: Signal::new(" ".to_string()),
            mail_smtp_password_set: Signal::new(true),
            mail_from_email: Signal::new(" noreply@example.com ".to_string()),
            mail_from_name: Signal::new(" Avenrixa ".to_string()),
            mail_link_base_url: Signal::new(" https://img.example.com ".to_string()),
        })
    }

    #[test]
    fn validate_allows_reusing_existing_smtp_password_when_user_is_present() {
        let harness = sample_form_harness();
        let form = harness.form;

        assert!(form.validate().is_ok());
    }

    #[test]
    fn build_update_request_trims_values_and_preserves_expected_version() {
        let harness = sample_form_harness();
        let form = harness.form;

        let request = form.build_update_request(Some("version-123".to_string()));

        assert_eq!(
            request.expected_settings_version.as_deref(),
            Some("version-123")
        );
        assert_eq!(request.site_name, "Avenrixa Console");
        assert_eq!(request.storage_backend, StorageBackendKind::Local);
        assert_eq!(request.local_storage_path, "/data/images");
        assert_eq!(request.mail_smtp_host, "smtp.example.com");
        assert_eq!(request.mail_smtp_port, Some(587));
        assert_eq!(request.mail_smtp_user.as_deref(), Some("mailer"));
        assert_eq!(request.mail_smtp_password, None);
        assert_eq!(request.mail_from_email, "noreply@example.com");
        assert_eq!(request.mail_from_name, "Avenrixa");
        assert_eq!(request.mail_link_base_url, "https://img.example.com");
    }

    #[test]
    fn build_update_request_omits_blank_optional_mail_fields() {
        let harness = TestSettingsFormHarness::new(|| SettingsFormState {
            site_name: Signal::new("Avenrixa".to_string()),
            storage_backend: Signal::new(StorageBackendKind::Local),
            local_storage_path: Signal::new("/data/images".to_string()),
            mail_enabled: Signal::new(false),
            mail_smtp_host: Signal::new(String::new()),
            mail_smtp_port: Signal::new("".to_string()),
            mail_smtp_user: Signal::new("   ".to_string()),
            mail_smtp_password: Signal::new("   ".to_string()),
            mail_smtp_password_set: Signal::new(false),
            mail_from_email: Signal::new(String::new()),
            mail_from_name: Signal::new("".to_string()),
            mail_link_base_url: Signal::new(String::new()),
        });
        let form = harness.form;

        let request = form.build_update_request(None);

        assert_eq!(request.expected_settings_version, None);
        assert_eq!(request.mail_smtp_port, None);
        assert_eq!(request.mail_smtp_user, None);
        assert_eq!(request.mail_smtp_password, None);
    }
}
