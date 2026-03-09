use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::config::{Builder as S3ConfigBuilder, Region};

use super::{CachedS3Client, StorageManager};
use crate::error::AppError;
use crate::runtime_settings::RuntimeSettings;

impl StorageManager {
    pub(super) async fn resolve_s3_client(
        &self,
        settings: &RuntimeSettings,
    ) -> Result<CachedS3Client, AppError> {
        let endpoint = required_s3_setting(settings.s3_endpoint.as_deref(), "endpoint")?;
        let region = required_s3_setting(settings.s3_region.as_deref(), "region")?;
        let bucket = required_s3_setting(settings.s3_bucket.as_deref(), "bucket")?;
        let access_key = required_s3_setting(settings.s3_access_key.as_deref(), "access key")?;
        let secret_key = required_s3_setting(settings.s3_secret_key.as_deref(), "secret key")?;
        let prefix = settings
            .s3_prefix
            .as_ref()
            .map(|v| v.trim_matches('/').to_string())
            .filter(|v| !v.is_empty());
        let signature = format!(
            "{}|{}|{}|{}|{}|{}",
            endpoint, region, bucket, access_key, secret_key, settings.s3_force_path_style
        );

        if let Some(cached) = self.s3_client_cache.read().await.as_ref()
            && cached.signature == signature
        {
            return Ok(cached.clone());
        }

        let creds = Credentials::new(access_key, secret_key, None, None, "runtime-settings");
        let mut builder = S3ConfigBuilder::new()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(region))
            .credentials_provider(creds)
            .endpoint_url(endpoint);
        if settings.s3_force_path_style {
            builder = builder.force_path_style(true);
        }
        let client = S3Client::from_conf(builder.build());

        let next = CachedS3Client {
            signature,
            bucket,
            prefix,
            client,
        };
        let mut guard = self.s3_client_cache.write().await;
        *guard = Some(next.clone());
        Ok(next)
    }
}

fn required_s3_setting(value: Option<&str>, name: &str) -> Result<String, AppError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            AppError::StorageBackendMisconfigured(format!("S3 配置缺少必填项: {}", name))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_s3_setting_rejects_missing_value() {
        assert!(matches!(
            required_s3_setting(None, "endpoint"),
            Err(AppError::StorageBackendMisconfigured(message))
                if message == "S3 配置缺少必填项: endpoint"
        ));
    }

    #[test]
    fn required_s3_setting_trims_whitespace() {
        let value = required_s3_setting(Some("  us-east-1  "), "region").unwrap();

        assert_eq!(value, "us-east-1");
    }
}
