use base64::Engine;

use crate::db::{DatabasePool, SITE_FAVICON_DATA_URL_SETTING_KEY, get_setting_value};
use crate::error::AppError;
use crate::models::{AdminSettingsConfig, storage_backend_kind_from_runtime};
use crate::runtime_settings::RuntimeSettings;

pub(crate) const MAX_FAVICON_BYTES: usize = 256 * 1024;

pub(crate) fn runtime_settings_to_admin_config(
    settings: &RuntimeSettings,
    favicon_configured: bool,
    restart_required: bool,
) -> AdminSettingsConfig {
    AdminSettingsConfig {
        site_name: settings.site_name.clone(),
        favicon_configured,
        storage_backend: storage_backend_kind_from_runtime(settings.storage_backend),
        local_storage_path: settings.local_storage_path.clone(),
        mail_enabled: settings.mail_enabled,
        mail_smtp_host: settings.mail_smtp_host.clone(),
        mail_smtp_port: settings.mail_smtp_port,
        mail_smtp_user: settings.mail_smtp_user.clone(),
        mail_smtp_password_set: settings
            .mail_smtp_password
            .as_ref()
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false),
        mail_from_email: settings.mail_from_email.clone(),
        mail_from_name: settings.mail_from_name.clone(),
        mail_link_base_url: settings.mail_link_base_url.clone(),
        restart_required,
        settings_version: settings.settings_version(),
    }
}

pub(crate) async fn favicon_is_configured(database: &DatabasePool) -> Result<bool, AppError> {
    Ok(
        get_setting_value(database, SITE_FAVICON_DATA_URL_SETTING_KEY)
            .await?
            .is_some_and(|value| !value.trim().is_empty()),
    )
}

pub(crate) fn validate_favicon_data_url(value: Option<String>) -> Result<Option<String>, AppError> {
    let Some(value) = value.map(|value| value.trim().to_string()) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }

    let Some((mime_prefix, encoded)) = value.split_once(";base64,") else {
        return Err(AppError::ValidationError(
            "网站图标必须使用 data URL(base64) 格式上传".to_string(),
        ));
    };
    let Some(mime) = mime_prefix.strip_prefix("data:") else {
        return Err(AppError::ValidationError("网站图标格式无效".to_string()));
    };

    if !matches!(
        mime,
        "image/x-icon"
            | "image/vnd.microsoft.icon"
            | "image/png"
            | "image/svg+xml"
            | "image/jpeg"
            | "image/webp"
    ) {
        return Err(AppError::ValidationError(
            "网站图标仅支持 ico/png/svg/jpeg/webp".to_string(),
        ));
    }

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| AppError::ValidationError("网站图标内容无法解析".to_string()))?;
    if bytes.is_empty() {
        return Err(AppError::ValidationError("网站图标不能为空".to_string()));
    }
    if bytes.len() > MAX_FAVICON_BYTES {
        return Err(AppError::ValidationError(format!(
            "网站图标不能超过 {} KB",
            MAX_FAVICON_BYTES / 1024
        )));
    }

    Ok(Some(format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::StorageBackendKind;
    use crate::runtime_settings::StorageBackend;
    use base64::Engine;

    fn sample_runtime_settings() -> RuntimeSettings {
        RuntimeSettings {
            site_name: "Avenrixa".to_string(),
            storage_backend: StorageBackend::Local,
            local_storage_path: "/data/images".to_string(),
            mail_enabled: true,
            mail_smtp_host: "smtp.example.com".to_string(),
            mail_smtp_port: 587,
            mail_smtp_user: Some("mailer".to_string()),
            mail_smtp_password: Some("secret".to_string()),
            mail_from_email: "noreply@example.com".to_string(),
            mail_from_name: "Avenrixa".to_string(),
            mail_link_base_url: "https://img.example.com/reset".to_string(),
        }
    }

    #[test]
    fn runtime_settings_to_admin_config_preserves_favicon_flag() {
        let config = runtime_settings_to_admin_config(&sample_runtime_settings(), true, false);

        assert_eq!(config.site_name, "Avenrixa");
        assert!(config.favicon_configured);
        assert_eq!(config.storage_backend, StorageBackendKind::Local);
        assert_eq!(config.local_storage_path, "/data/images");
        assert_eq!(config.mail_smtp_user.as_deref(), Some("mailer"));
        assert!(config.mail_smtp_password_set);
        assert!(!config.settings_version.is_empty());
    }

    #[test]
    fn validate_favicon_data_url_accepts_supported_payload_and_normalizes_output() {
        let payload = base64::engine::general_purpose::STANDARD.encode([0_u8, 1, 2, 3]);
        let normalized =
            validate_favicon_data_url(Some(format!("data:image/png;base64,{payload}")))
                .expect("favicon should validate")
                .expect("favicon should be preserved");

        assert_eq!(normalized, format!("data:image/png;base64,{payload}"));
    }

    #[test]
    fn validate_favicon_data_url_rejects_unsupported_mime() {
        let payload = base64::engine::general_purpose::STANDARD.encode([0_u8, 1, 2, 3]);
        let err = validate_favicon_data_url(Some(format!("data:text/plain;base64,{payload}")))
            .expect_err("unsupported mime should fail");

        assert!(
            matches!(err, AppError::ValidationError(message) if message.contains("网站图标仅支持"))
        );
    }

    #[test]
    fn validate_favicon_data_url_rejects_oversized_payload() {
        let payload =
            base64::engine::general_purpose::STANDARD.encode(vec![7_u8; MAX_FAVICON_BYTES + 1]);
        let err = validate_favicon_data_url(Some(format!("data:image/png;base64,{payload}")))
            .expect_err("oversized favicon should fail");

        assert!(
            matches!(err, AppError::ValidationError(message) if message.contains("网站图标不能超过"))
        );
    }
}
