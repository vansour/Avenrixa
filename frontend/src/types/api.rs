use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 登录请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

/// 用户响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub email: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

/// 分页参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            page_size: Some(20),
            tag: None,
        }
    }
}

/// 分页响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paginated<T> {
    pub data: Vec<T>,
    pub page: i32,
    pub page_size: i32,
    pub total: i64,
    pub has_next: bool,
}

/// 批量删除请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRequest {
    pub image_keys: Vec<String>,
    pub permanent: bool,
}

/// 恢复请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreRequest {
    pub image_keys: Vec<String>,
}

/// 设置过期请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetExpiryRequest {
    pub expires_at: Option<DateTime<Utc>>,
}

/// 更新图片请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateImageRequest {
    pub tags: Option<Vec<String>>,
}

/// 修改密码请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    pub current_password: String,
    pub new_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetRequest {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetConfirmRequest {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailVerificationConfirmRequest {
    pub token: String,
}

/// 管理员设置配置（结构化）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdminSettingsConfig {
    pub site_name: String,
    pub storage_backend: String,
    pub local_storage_path: String,
    pub mail_enabled: bool,
    pub mail_smtp_host: String,
    pub mail_smtp_port: u16,
    pub mail_smtp_user: Option<String>,
    pub mail_smtp_password_set: bool,
    pub mail_from_email: String,
    pub mail_from_name: String,
    pub mail_link_base_url: String,
    pub s3_endpoint: Option<String>,
    pub s3_region: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_prefix: Option<String>,
    pub s3_access_key: Option<String>,
    pub s3_secret_key_set: bool,
    pub s3_force_path_style: bool,
    pub restart_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BootstrapStatusResponse {
    pub mode: String,
    pub database_kind: String,
    pub database_configured: bool,
    pub database_url_masked: Option<String>,
    pub cache_configured: bool,
    pub cache_url_masked: Option<String>,
    pub restart_required: bool,
    pub runtime_error: Option<String>,
}

/// 更新管理员设置请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAdminSettingsConfigRequest {
    pub site_name: String,
    pub storage_backend: String,
    pub local_storage_path: String,
    pub mail_enabled: bool,
    pub mail_smtp_host: String,
    pub mail_smtp_port: Option<u16>,
    pub mail_smtp_user: Option<String>,
    pub mail_smtp_password: Option<String>,
    pub mail_from_email: String,
    pub mail_from_name: String,
    pub mail_link_base_url: String,
    pub s3_endpoint: Option<String>,
    pub s3_region: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_prefix: Option<String>,
    pub s3_access_key: Option<String>,
    pub s3_secret_key: Option<String>,
    pub s3_force_path_style: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBootstrapDatabaseConfigRequest {
    pub database_kind: String,
    pub database_url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateBootstrapDatabaseConfigResponse {
    pub database_kind: String,
    pub database_configured: bool,
    pub database_url_masked: String,
    pub restart_required: bool,
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
pub struct ComponentStatus {
    pub status: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthMetrics {
    pub images_count: i64,
    pub users_count: i64,
    pub storage_used_mb: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub database: ComponentStatus,
    #[serde(alias = "redis")]
    pub cache: ComponentStatus,
    pub storage: ComponentStatus,
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupResponse {
    pub filename: String,
    pub created_at: DateTime<Utc>,
    pub semantics: BackupSemantics,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupFileSummary {
    pub filename: String,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
    pub semantics: BackupSemantics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupSemantics {
    pub database_family: String,
    pub backup_kind: String,
    pub backup_scope: String,
    pub restore_mode: String,
    pub artifact_layout: String,
    pub ui_restore_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupRestoreStorageSummary {
    pub storage_backend: String,
    pub local_storage_path: String,
    pub s3_endpoint: Option<String>,
    pub s3_region: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_prefix: Option<String>,
    pub s3_force_path_style: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupObjectRollbackAnchor {
    pub strategy: String,
    pub checkpoint_at: DateTime<Utc>,
    pub local_storage_path: Option<String>,
    pub s3_endpoint: Option<String>,
    pub s3_region: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_prefix: Option<String>,
    pub s3_force_path_style: bool,
    pub s3_bucket_versioning_status: Option<String>,
    pub capture_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupRestorePrecheckResponse {
    pub eligible: bool,
    pub filename: String,
    pub backup_created_at: DateTime<Utc>,
    pub backup_size_bytes: u64,
    pub current_database_kind: String,
    pub backup_database_kind: String,
    pub semantics: BackupSemantics,
    pub integrity_check_passed: bool,
    pub app_installed: bool,
    pub has_admin: bool,
    pub storage_compatible: bool,
    pub current_storage: BackupRestoreStorageSummary,
    pub backup_storage: BackupRestoreStorageSummary,
    pub object_rollback_anchor: Option<BackupObjectRollbackAnchor>,
    pub warnings: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingBackupRestore {
    pub filename: String,
    pub database_kind: String,
    pub semantics: BackupSemantics,
    pub requested_by_user_id: String,
    pub requested_by_email: String,
    pub scheduled_at: DateTime<Utc>,
    pub backup_created_at: DateTime<Utc>,
    pub backup_size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupRestoreResult {
    pub status: String,
    pub filename: String,
    pub database_kind: String,
    pub semantics: BackupSemantics,
    pub message: String,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: DateTime<Utc>,
    pub rollback_filename: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupRestoreStatusResponse {
    pub pending: Option<PendingBackupRestore>,
    pub last_result: Option<BackupRestoreResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupRestoreScheduleResponse {
    pub scheduled: bool,
    pub restart_required: bool,
    pub pending: PendingBackupRestore,
    pub precheck: BackupRestorePrecheckResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdminUserSummary {
    pub id: String,
    pub email: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUpdateRequest {
    pub role: Option<String>,
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
