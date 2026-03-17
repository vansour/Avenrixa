use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::Ordering;
use std::time::Duration;

use aws_sdk_s3::primitives::ByteStream;
use tokio::fs;
use uuid::Uuid;

use super::path::build_s3_object_key;
use super::{S3HealthProbeOutcome, StorageManager};
use crate::error::AppError;
use crate::models::{ComponentStatus, HealthState};
use crate::runtime_settings::{RuntimeSettings, StorageBackend};

const S3_HEALTH_PROBE_TIMEOUT: Duration = Duration::from_secs(3);
const S3_HEALTH_PROBE_CACHE_TTL: Duration = Duration::from_secs(30);

impl StorageManager {
    pub fn new(active_settings: RuntimeSettings) -> Self {
        Self {
            active_settings: Arc::new(std::sync::RwLock::new(active_settings)),
            s3_client_cache: Arc::new(tokio::sync::RwLock::new(None)),
            s3_health_cache: Arc::new(tokio::sync::RwLock::new(None)),
            #[cfg(test)]
            fail_next_apply: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            #[cfg(test)]
            s3_health_probe_override: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    pub fn active_settings(&self) -> RuntimeSettings {
        self.active_settings
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn restart_required(&self, settings: &RuntimeSettings) -> bool {
        self.active_settings().storage_settings() != settings.storage_settings()
    }

    pub async fn validate_runtime_settings(
        &self,
        settings: &RuntimeSettings,
    ) -> Result<(), AppError> {
        if settings.storage_backend == StorageBackend::Local {
            fs::create_dir_all(&settings.local_storage_path).await?;
            let images_perms = PermissionsExt::from_mode(0o755);
            let _ = fs::set_permissions(&settings.local_storage_path, images_perms).await;
            return Ok(());
        }

        Self::build_s3_client(settings)?;
        Ok(())
    }

    pub async fn apply_runtime_settings(&self, settings: RuntimeSettings) -> Result<(), AppError> {
        self.validate_runtime_settings(&settings).await?;

        #[cfg(test)]
        if self.fail_next_apply.swap(false, Ordering::SeqCst) {
            return Err(AppError::Internal(anyhow::anyhow!(
                "Injected runtime settings apply failure"
            )));
        }

        {
            let mut guard = self
                .active_settings
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *guard = settings;
        }

        *self.s3_client_cache.write().await = None;
        *self.s3_health_cache.write().await = None;
        Ok(())
    }

    pub async fn health_component_status(&self) -> ComponentStatus {
        let settings = self.active_settings();
        match settings.storage_backend {
            StorageBackend::Local => self.local_health_status(&settings).await,
            StorageBackend::S3 => self.s3_health_status(&settings).await,
        }
    }

    pub async fn test_s3_settings(&self, settings: &RuntimeSettings) -> Result<(), AppError> {
        let client = Self::build_s3_client(settings)?;
        let probe_file_key = format!(".vansour-s3-probe-{}", Uuid::new_v4().simple());
        let object_key = build_s3_object_key(client.prefix.as_deref(), &probe_file_key);

        client
            .client
            .put_object()
            .bucket(client.bucket.clone())
            .key(object_key.clone())
            .body(ByteStream::from_static(b"vansour-s3-probe"))
            .send()
            .await
            .map_err(|e| AppError::ValidationError(format!("S3 上传测试失败: {}", e)))?;

        client
            .client
            .get_object()
            .bucket(client.bucket.clone())
            .key(object_key.clone())
            .send()
            .await
            .map_err(|e| AppError::ValidationError(format!("S3 读取测试失败: {}", e)))?;

        client
            .client
            .delete_object()
            .bucket(client.bucket)
            .key(object_key)
            .send()
            .await
            .map_err(|e| AppError::ValidationError(format!("S3 清理测试文件失败: {}", e)))?;

        Ok(())
    }

    pub fn cache_hint(&self, file_key: &str) -> String {
        format!("storage://{}", file_key)
    }

    async fn local_health_status(&self, settings: &RuntimeSettings) -> ComponentStatus {
        match fs::metadata(&settings.local_storage_path).await {
            Ok(_) => ComponentStatus {
                status: HealthState::Healthy,
                message: Some("配置=正常 | 路径访问=正常 | 读写=本地文件系统".to_string()),
            },
            Err(error) => ComponentStatus::unhealthy(format!(
                "配置=正常 | 路径访问=失败: {} | 读写=不可用",
                error
            )),
        }
    }

    async fn s3_health_status(&self, settings: &RuntimeSettings) -> ComponentStatus {
        let client = match Self::build_s3_client(settings) {
            Ok(client) => client,
            Err(error) => {
                return ComponentStatus::unhealthy(format!(
                    "配置=失败: {} | 远端探测=未执行 | 读写=不可用",
                    error
                ));
            }
        };

        match self.cached_s3_health_probe(&client).await {
            S3HealthProbeOutcome::Healthy => ComponentStatus {
                status: HealthState::Healthy,
                message: Some("配置=正常 | 远端探测=正常 | 读写=未执行写探测".to_string()),
            },
            S3HealthProbeOutcome::Timeout(message) => ComponentStatus::degraded(format!(
                "配置=正常 | 远端探测=超时: {} | 读写=未执行写探测",
                message
            )),
            S3HealthProbeOutcome::Failure(message) => ComponentStatus::unhealthy(format!(
                "配置=正常 | 远端探测=失败: {} | 读写=未执行写探测",
                message
            )),
        }
    }

    async fn cached_s3_health_probe(&self, client: &super::CachedS3Client) -> S3HealthProbeOutcome {
        #[cfg(test)]
        if let Some(outcome) = self.s3_health_probe_override.read().await.clone() {
            return outcome;
        }

        if let Some(cached) = self.s3_health_cache.read().await.as_ref()
            && cached.signature == client.signature
            && cached.checked_at.elapsed() < S3_HEALTH_PROBE_CACHE_TTL
        {
            return cached.outcome.clone();
        }

        let outcome = Self::probe_s3_health(client).await;
        let mut guard = self.s3_health_cache.write().await;
        *guard = Some(super::CachedS3HealthProbe {
            signature: client.signature.clone(),
            checked_at: std::time::Instant::now(),
            outcome: outcome.clone(),
        });
        outcome
    }

    async fn probe_s3_health(client: &super::CachedS3Client) -> S3HealthProbeOutcome {
        match tokio::time::timeout(
            S3_HEALTH_PROBE_TIMEOUT,
            client
                .client
                .head_bucket()
                .bucket(client.bucket.clone())
                .send(),
        )
        .await
        {
            Ok(Ok(_)) => S3HealthProbeOutcome::Healthy,
            Ok(Err(error)) => S3HealthProbeOutcome::Failure(error.to_string()),
            Err(_) => S3HealthProbeOutcome::Timeout(format!(
                "{} 秒内未收到对象存储响应",
                S3_HEALTH_PROBE_TIMEOUT.as_secs()
            )),
        }
    }

    #[cfg(test)]
    pub fn fail_next_apply_for_tests(&self) {
        self.fail_next_apply.store(true, Ordering::SeqCst);
    }

    #[cfg(test)]
    pub async fn set_s3_health_probe_override_for_tests(
        &self,
        outcome: Option<S3HealthProbeOutcome>,
    ) {
        *self.s3_health_probe_override.write().await = outcome;
        *self.s3_health_cache.write().await = None;
    }

    #[cfg(test)]
    pub async fn force_s3_health_probe_success_for_tests(&self) {
        self.set_s3_health_probe_override_for_tests(Some(S3HealthProbeOutcome::Healthy))
            .await;
    }

    #[cfg(test)]
    pub async fn force_s3_health_probe_failure_for_tests(&self, message: &str) {
        self.set_s3_health_probe_override_for_tests(Some(S3HealthProbeOutcome::Failure(
            message.to_string(),
        )))
        .await;
    }

    #[cfg(test)]
    pub async fn force_s3_health_probe_timeout_for_tests(&self, message: &str) {
        self.set_s3_health_probe_override_for_tests(Some(S3HealthProbeOutcome::Timeout(
            message.to_string(),
        )))
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_runtime_settings(local_storage_path: String) -> RuntimeSettings {
        RuntimeSettings {
            site_name: "Avenrixa".to_string(),
            storage_backend: StorageBackend::Local,
            local_storage_path,
            mail_enabled: false,
            mail_smtp_host: String::new(),
            mail_smtp_port: 587,
            mail_smtp_user: None,
            mail_smtp_password: None,
            mail_from_email: String::new(),
            mail_from_name: String::new(),
            mail_link_base_url: String::new(),
            s3_endpoint: None,
            s3_region: None,
            s3_bucket: None,
            s3_prefix: None,
            s3_access_key: None,
            s3_secret_key: None,
            s3_force_path_style: true,
        }
    }

    fn sample_s3_runtime_settings() -> RuntimeSettings {
        RuntimeSettings {
            site_name: "Avenrixa".to_string(),
            storage_backend: StorageBackend::S3,
            local_storage_path: "/tmp/avenrixa".to_string(),
            mail_enabled: false,
            mail_smtp_host: String::new(),
            mail_smtp_port: 587,
            mail_smtp_user: None,
            mail_smtp_password: None,
            mail_from_email: String::new(),
            mail_from_name: String::new(),
            mail_link_base_url: String::new(),
            s3_endpoint: Some("https://example.r2.cloudflarestorage.com".to_string()),
            s3_region: Some("auto".to_string()),
            s3_bucket: Some("images".to_string()),
            s3_prefix: Some("uploads".to_string()),
            s3_access_key: Some("access".to_string()),
            s3_secret_key: Some("secret".to_string()),
            s3_force_path_style: false,
        }
    }

    #[tokio::test]
    async fn apply_runtime_settings_updates_local_path_and_creates_directory() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let initial_path = temp_dir.path().join("initial");
        let next_path = temp_dir.path().join("mounted").join("images");
        let manager = StorageManager::new(sample_runtime_settings(
            initial_path.to_string_lossy().into(),
        ));

        manager
            .apply_runtime_settings(sample_runtime_settings(next_path.to_string_lossy().into()))
            .await
            .expect("runtime settings should apply");

        assert_eq!(
            manager.active_settings().local_storage_path,
            next_path.to_string_lossy().to_string()
        );
        assert!(
            tokio::fs::try_exists(&next_path)
                .await
                .expect("existence check should succeed")
        );
    }

    #[tokio::test]
    async fn health_status_reports_cached_s3_probe_failures() {
        let manager = StorageManager::new(sample_s3_runtime_settings());
        manager
            .force_s3_health_probe_failure_for_tests("access denied")
            .await;

        let status = manager.health_component_status().await;

        assert_eq!(status.status, HealthState::Unhealthy);
        assert!(
            status
                .message
                .as_deref()
                .is_some_and(|message| message.contains("远端探测=失败: access denied"))
        );
    }

    #[tokio::test]
    async fn health_status_reports_s3_probe_success() {
        let manager = StorageManager::new(sample_s3_runtime_settings());
        manager.force_s3_health_probe_success_for_tests().await;

        let status = manager.health_component_status().await;

        assert_eq!(status.status, HealthState::Healthy);
        assert!(
            status
                .message
                .as_deref()
                .is_some_and(|message| message.contains("远端探测=正常"))
        );
    }

    #[tokio::test]
    async fn health_status_reports_s3_probe_timeouts_as_degraded() {
        let manager = StorageManager::new(sample_s3_runtime_settings());
        manager
            .force_s3_health_probe_timeout_for_tests("3 秒内未收到对象存储响应")
            .await;

        let status = manager.health_component_status().await;

        assert_eq!(status.status, HealthState::Degraded);
        assert!(
            status
                .message
                .as_deref()
                .is_some_and(|message| message.contains("远端探测=超时"))
        );
    }
}
