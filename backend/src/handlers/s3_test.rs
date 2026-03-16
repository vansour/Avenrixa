use crate::error::AppError;
use crate::models::{TestS3StorageConfigRequest, TestS3StorageConfigResponse};
use crate::runtime_settings::{RuntimeSettings, StorageBackend};

fn normalize_option(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_prefix(value: Option<String>) -> Option<String> {
    normalize_option(value)
        .map(|value| value.trim_matches('/').to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn build_s3_test_settings(
    current: RuntimeSettings,
    req: TestS3StorageConfigRequest,
) -> Result<RuntimeSettings, AppError> {
    let mut settings = current;
    settings.storage_backend = StorageBackend::S3;
    settings.s3_endpoint = normalize_option(req.s3_endpoint);
    settings.s3_region = normalize_option(req.s3_region);
    settings.s3_bucket = normalize_option(req.s3_bucket);
    settings.s3_prefix = normalize_prefix(req.s3_prefix);
    settings.s3_access_key = normalize_option(req.s3_access_key);
    settings.s3_force_path_style = req.s3_force_path_style.unwrap_or(true);

    if let Some(secret_key) = normalize_option(req.s3_secret_key) {
        settings.s3_secret_key = Some(secret_key);
    } else if !req.s3_secret_key_set {
        settings.s3_secret_key = None;
    }

    if settings
        .s3_endpoint
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
        || settings
            .s3_region
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        || settings
            .s3_bucket
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        || settings
            .s3_access_key
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        || settings
            .s3_secret_key
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err(AppError::ValidationError(
            "S3 模式下必须完整填写 endpoint、region、bucket、access key 和 secret key".to_string(),
        ));
    }

    Ok(settings)
}

pub(crate) fn s3_test_success_response() -> TestS3StorageConfigResponse {
    TestS3StorageConfigResponse {
        message: "S3 连通性测试通过，可以确认当前配置".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_runtime_settings() -> RuntimeSettings {
        RuntimeSettings {
            site_name: "Vansour Image".to_string(),
            storage_backend: StorageBackend::Local,
            local_storage_path: "/data/images".to_string(),
            mail_enabled: false,
            mail_smtp_host: String::new(),
            mail_smtp_port: 587,
            mail_smtp_user: None,
            mail_smtp_password: None,
            mail_from_email: String::new(),
            mail_from_name: String::new(),
            mail_link_base_url: String::new(),
            s3_endpoint: Some("https://old.example.com".to_string()),
            s3_region: Some("us-east-1".to_string()),
            s3_bucket: Some("old-bucket".to_string()),
            s3_prefix: Some("old-prefix".to_string()),
            s3_access_key: Some("old-access".to_string()),
            s3_secret_key: Some("old-secret".to_string()),
            s3_force_path_style: true,
        }
    }

    #[test]
    fn build_s3_test_settings_reuses_existing_secret_when_flagged() {
        let settings = build_s3_test_settings(
            sample_runtime_settings(),
            TestS3StorageConfigRequest {
                s3_endpoint: Some(" https://minio.example.com ".to_string()),
                s3_region: Some(" us-east-1 ".to_string()),
                s3_bucket: Some(" images ".to_string()),
                s3_prefix: Some(" /uploads/2026/ ".to_string()),
                s3_access_key: Some(" access ".to_string()),
                s3_secret_key: None,
                s3_secret_key_set: true,
                s3_force_path_style: Some(false),
            },
        )
        .expect("settings should be built");

        assert_eq!(settings.storage_backend, StorageBackend::S3);
        assert_eq!(
            settings.s3_endpoint.as_deref(),
            Some("https://minio.example.com")
        );
        assert_eq!(settings.s3_region.as_deref(), Some("us-east-1"));
        assert_eq!(settings.s3_bucket.as_deref(), Some("images"));
        assert_eq!(settings.s3_prefix.as_deref(), Some("uploads/2026"));
        assert_eq!(settings.s3_access_key.as_deref(), Some("access"));
        assert_eq!(settings.s3_secret_key.as_deref(), Some("old-secret"));
        assert!(!settings.s3_force_path_style);
    }

    #[test]
    fn build_s3_test_settings_requires_complete_credentials() {
        let error = build_s3_test_settings(
            sample_runtime_settings(),
            TestS3StorageConfigRequest {
                s3_endpoint: Some("https://minio.example.com".to_string()),
                s3_region: Some("us-east-1".to_string()),
                s3_bucket: Some("images".to_string()),
                s3_prefix: None,
                s3_access_key: Some("access".to_string()),
                s3_secret_key: None,
                s3_secret_key_set: false,
                s3_force_path_style: Some(true),
            },
        )
        .expect_err("missing secret should be rejected");

        assert!(matches!(
            error,
            AppError::ValidationError(message)
                if message == "S3 模式下必须完整填写 endpoint、region、bucket、access key 和 secret key"
        ));
    }
}
