use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::auth::UserResponse;
use crate::common::{HealthState, StorageBackendKind, UserRole};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdminSettingsConfig {
    pub site_name: String,
    pub storage_backend: StorageBackendKind,
    pub local_storage_path: String,
    pub mail_enabled: bool,
    pub mail_smtp_host: String,
    pub mail_smtp_port: u16,
    pub mail_smtp_user: Option<String>,
    pub mail_smtp_password_set: bool,
    pub mail_from_email: String,
    pub mail_from_name: String,
    pub mail_link_base_url: String,
    pub restart_required: bool,
    #[serde(default)]
    pub settings_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstallStatusResponse {
    pub installed: bool,
    pub has_admin: bool,
    pub favicon_configured: bool,
    pub config: AdminSettingsConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageDirectoryEntry {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageDirectoryBrowseResponse {
    pub current_path: String,
    pub parent_path: Option<String>,
    pub directories: Vec<StorageDirectoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAdminSettingsConfigRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_settings_version: Option<String>,
    pub site_name: String,
    pub storage_backend: StorageBackendKind,
    pub local_storage_path: String,
    pub mail_enabled: bool,
    pub mail_smtp_host: String,
    pub mail_smtp_port: Option<u16>,
    pub mail_smtp_user: Option<String>,
    pub mail_smtp_password: Option<String>,
    pub mail_from_email: String,
    pub mail_from_name: String,
    pub mail_link_base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallBootstrapRequest {
    pub admin_email: String,
    pub admin_password: String,
    pub favicon_data_url: Option<String>,
    pub config: UpdateAdminSettingsConfigRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallBootstrapResponse {
    pub user: UserResponse,
    pub favicon_configured: bool,
    pub config: AdminSettingsConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdminUserSummary {
    pub id: String,
    pub email: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
    pub editable: bool,
    pub sensitive: bool,
    pub masked: bool,
    pub requires_confirmation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSettingRequest {
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub user_id: Option<String>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditLogResponse {
    pub data: Vec<AuditLog>,
    pub page: i32,
    pub page_size: i32,
    pub total: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentStatus {
    pub status: HealthState,
    pub message: Option<String>,
}

impl ComponentStatus {
    pub fn healthy() -> Self {
        Self {
            status: HealthState::Healthy,
            message: None,
        }
    }

    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            status: HealthState::Unhealthy,
            message: Some(message.into()),
        }
    }

    pub fn degraded(message: impl Into<String>) -> Self {
        Self {
            status: HealthState::Degraded,
            message: Some(message.into()),
        }
    }

    pub fn disabled(message: impl Into<String>) -> Self {
        Self {
            status: HealthState::Disabled,
            message: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeOperationMetrics {
    pub total_successes: u64,
    pub total_failures: u64,
    pub last_duration_ms: Option<u64>,
    pub average_duration_ms: Option<u64>,
    pub max_duration_ms: Option<u64>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackgroundTaskMetrics {
    pub task_name: String,
    pub total_runs: u64,
    pub total_failures: u64,
    pub consecutive_failures: u64,
    pub last_duration_ms: Option<u64>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeBacklogMetrics {
    pub storage_cleanup_pending: i64,
    pub storage_cleanup_retrying: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeObservabilitySnapshot {
    pub audit_writes: RuntimeOperationMetrics,
    pub auth_refresh: RuntimeOperationMetrics,
    pub image_processing: RuntimeOperationMetrics,
    pub backups: RuntimeOperationMetrics,
    pub background_tasks: Vec<BackgroundTaskMetrics>,
    pub backlog: RuntimeBacklogMetrics,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthMetrics {
    pub images_count: Option<i64>,
    pub users_count: Option<i64>,
    pub storage_used_mb: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: HealthState,
    pub timestamp: DateTime<Utc>,
    pub database: ComponentStatus,
    pub cache: ComponentStatus,
    pub storage: ComponentStatus,
    pub observability: ComponentStatus,
    pub version: Option<String>,
    pub uptime_seconds: Option<u64>,
    pub metrics: Option<HealthMetrics>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemStats {
    pub total_users: i64,
    pub total_images: i64,
    pub total_storage: i64,
    pub total_views: i64,
    pub images_last_24h: i64,
    pub images_last_7d: i64,
    pub runtime: RuntimeObservabilitySnapshot,
}
