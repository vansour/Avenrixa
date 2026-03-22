use crate::error::AppError;
use crate::models::{
    StorageBackendKind, UpdateAdminSettingsConfigRequest, runtime_storage_backend_from_kind,
    storage_backend_kind_from_runtime,
};
use lettre::Address;
use reqwest::Url;

use super::model::{RuntimeSettings, StorageBackend};

fn trim_to_option(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn validate_enabled_mail_settings(settings: &RuntimeSettings) -> Result<(), AppError> {
    if settings.mail_smtp_host.trim().is_empty() {
        return Err(AppError::ValidationError(
            "启用邮件服务时 SMTP 主机不能为空".to_string(),
        ));
    }
    if settings.mail_smtp_port == 0 {
        return Err(AppError::ValidationError(
            "启用邮件服务时 SMTP 端口必须大于 0".to_string(),
        ));
    }
    if settings.mail_from_email.trim().is_empty() {
        return Err(AppError::ValidationError(
            "启用邮件服务时发件邮箱不能为空".to_string(),
        ));
    }
    settings
        .mail_from_email
        .parse::<Address>()
        .map_err(|_| AppError::ValidationError("发件邮箱格式无效".to_string()))?;

    if settings.mail_link_base_url.trim().is_empty() {
        return Err(AppError::ValidationError(
            "启用邮件服务时站点访问地址不能为空".to_string(),
        ));
    }
    let link_url = Url::parse(&settings.mail_link_base_url)
        .map_err(|_| AppError::ValidationError("站点访问地址格式无效".to_string()))?;
    if !matches!(link_url.scheme(), "http" | "https") {
        return Err(AppError::ValidationError(
            "站点访问地址必须是 http 或 https".to_string(),
        ));
    }

    let has_smtp_user = settings
        .mail_smtp_user
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty());
    let has_smtp_password = settings
        .mail_smtp_password
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty());
    if has_smtp_user != has_smtp_password {
        return Err(AppError::ValidationError(
            "SMTP 用户名和密码必须同时配置或同时留空".to_string(),
        ));
    }

    Ok(())
}

pub(crate) fn validate_and_merge(
    mut current: RuntimeSettings,
    req: UpdateAdminSettingsConfigRequest,
) -> Result<RuntimeSettings, AppError> {
    let site_name = req.site_name.trim();
    if site_name.is_empty() || site_name.chars().count() > 120 {
        return Err(AppError::ValidationError(
            "网站名称不能为空且不能超过 120 个字符".to_string(),
        ));
    }
    current.site_name = site_name.to_string();

    current.storage_backend = runtime_storage_backend_from_kind(req.storage_backend)
        .ok_or_else(|| AppError::ValidationError("存储后端必须是 local".to_string()))?;

    let local_path = req.local_storage_path.trim();
    if current.storage_backend == StorageBackend::Local && local_path.is_empty() {
        return Err(AppError::ValidationError(
            "本地存储路径不能为空".to_string(),
        ));
    }
    current.local_storage_path = local_path.to_string();

    let previous_mail_user = current.mail_smtp_user.clone();
    current.mail_enabled = req.mail_enabled;
    current.mail_smtp_host = req.mail_smtp_host.trim().to_string();
    if let Some(port) = req.mail_smtp_port {
        current.mail_smtp_port = port;
    }
    current.mail_smtp_user = trim_to_option(req.mail_smtp_user);
    let mail_user_changed = current.mail_smtp_user != previous_mail_user;
    match trim_to_option(req.mail_smtp_password) {
        Some(password) => {
            current.mail_smtp_password = Some(password);
        }
        None => {
            if current.mail_smtp_user.is_none() || mail_user_changed {
                current.mail_smtp_password = None;
            }
        }
    }
    current.mail_from_email = req.mail_from_email.trim().to_string();
    current.mail_from_name = req.mail_from_name.trim().to_string();
    current.mail_link_base_url = req.mail_link_base_url.trim().to_string();

    if current.mail_enabled {
        validate_enabled_mail_settings(&current)?;
    }

    Ok(current)
}

pub fn validate_raw_setting_update(
    current: RuntimeSettings,
    key: &str,
    value: &str,
) -> Result<RuntimeSettings, AppError> {
    let mut req = UpdateAdminSettingsConfigRequest {
        expected_settings_version: None,
        site_name: current.site_name.clone(),
        storage_backend: storage_backend_kind_from_runtime(current.storage_backend),
        local_storage_path: current.local_storage_path.clone(),
        mail_enabled: current.mail_enabled,
        mail_smtp_host: current.mail_smtp_host.clone(),
        mail_smtp_port: Some(current.mail_smtp_port),
        mail_smtp_user: current.mail_smtp_user.clone(),
        mail_smtp_password: current.mail_smtp_password.clone(),
        mail_from_email: current.mail_from_email.clone(),
        mail_from_name: current.mail_from_name.clone(),
        mail_link_base_url: current.mail_link_base_url.clone(),
    };

    match key {
        super::model::SETTING_SITE_NAME => {
            req.site_name = value.to_string();
        }
        super::model::SETTING_STORAGE_BACKEND => {
            req.storage_backend = StorageBackendKind::parse(value.trim());
        }
        super::model::SETTING_LOCAL_STORAGE_PATH => {
            req.local_storage_path = value.to_string();
        }
        super::model::SETTING_MAIL_ENABLED => {
            let normalized = value.trim().to_ascii_lowercase();
            req.mail_enabled = match normalized.as_str() {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(AppError::ValidationError(
                        "mail_enabled 只能填写 true 或 false".to_string(),
                    ));
                }
            };
        }
        super::model::SETTING_MAIL_SMTP_HOST => {
            req.mail_smtp_host = value.to_string();
        }
        super::model::SETTING_MAIL_SMTP_PORT => {
            req.mail_smtp_port = Some(value.trim().parse::<u16>().map_err(|_| {
                AppError::ValidationError("mail_smtp_port 必须是 1-65535 的整数".to_string())
            })?);
        }
        super::model::SETTING_MAIL_SMTP_USER => {
            req.mail_smtp_user = Some(value.to_string());
            if value.trim().is_empty() {
                req.mail_smtp_password = Some(String::new());
            }
        }
        super::model::SETTING_MAIL_FROM_EMAIL => {
            req.mail_from_email = value.to_string();
        }
        super::model::SETTING_MAIL_FROM_NAME => {
            req.mail_from_name = value.to_string();
        }
        super::model::SETTING_MAIL_LINK_BASE_URL => {
            req.mail_link_base_url = value.to_string();
        }
        _ => {
            return Err(AppError::ValidationError(
                "该设置项不允许通过高级设置修改".to_string(),
            ));
        }
    }

    validate_and_merge(current, req)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_runtime_settings() -> RuntimeSettings {
        RuntimeSettings {
            site_name: "Avenrixa".to_string(),
            storage_backend: StorageBackend::Local,
            local_storage_path: "/data/images".to_string(),
            mail_enabled: false,
            mail_smtp_host: "smtp.example.com".to_string(),
            mail_smtp_port: 587,
            mail_smtp_user: None,
            mail_smtp_password: None,
            mail_from_email: "noreply@example.com".to_string(),
            mail_from_name: "Avenrixa".to_string(),
            mail_link_base_url: "https://img.example.com".to_string(),
        }
    }

    fn local_request() -> UpdateAdminSettingsConfigRequest {
        UpdateAdminSettingsConfigRequest {
            expected_settings_version: None,
            site_name: "New Site".to_string(),
            storage_backend: StorageBackendKind::Local,
            local_storage_path: "/srv/images".to_string(),
            mail_enabled: false,
            mail_smtp_host: "smtp.example.com".to_string(),
            mail_smtp_port: Some(587),
            mail_smtp_user: None,
            mail_smtp_password: None,
            mail_from_email: "noreply@example.com".to_string(),
            mail_from_name: "Avenrixa".to_string(),
            mail_link_base_url: "https://img.example.com".to_string(),
        }
    }

    #[test]
    fn validate_and_merge_rejects_invalid_storage_backend() {
        let mut req = local_request();
        req.storage_backend = StorageBackendKind::Unknown;

        assert!(matches!(
            validate_and_merge(base_runtime_settings(), req),
            Err(AppError::ValidationError(message))
                if message == "存储后端必须是 local"
        ));
    }

    #[test]
    fn validate_and_merge_accepts_enabled_mail_with_complete_settings() {
        let mut req = local_request();
        req.mail_enabled = true;
        req.mail_smtp_user = Some("mailer".to_string());
        req.mail_smtp_password = Some("secret".to_string());

        let merged = validate_and_merge(base_runtime_settings(), req).unwrap();

        assert!(merged.mail_enabled);
        assert_eq!(merged.mail_smtp_user.as_deref(), Some("mailer"));
        assert_eq!(merged.mail_smtp_password.as_deref(), Some("secret"));
    }

    #[test]
    fn validate_and_merge_rejects_enabled_mail_with_invalid_link() {
        let mut req = local_request();
        req.mail_enabled = true;
        req.mail_link_base_url = "mailto:reset@example.com".to_string();

        assert!(matches!(
            validate_and_merge(base_runtime_settings(), req),
            Err(AppError::ValidationError(message))
                if message == "站点访问地址必须是 http 或 https"
        ));
    }

    #[test]
    fn validate_raw_setting_update_supports_runtime_site_name() {
        let current = base_runtime_settings();

        let updated = validate_raw_setting_update(
            current,
            super::super::model::SETTING_SITE_NAME,
            "  New Admin Site  ",
        )
        .unwrap();

        assert_eq!(updated.site_name, "New Admin Site");
    }

    #[test]
    fn validate_raw_setting_update_supports_runtime_mail_enabled() {
        let current = base_runtime_settings();

        let updated =
            validate_raw_setting_update(current, super::super::model::SETTING_MAIL_ENABLED, "true")
                .unwrap();

        assert!(updated.mail_enabled);
    }
}
