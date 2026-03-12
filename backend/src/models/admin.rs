use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::DatabaseKind;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<Uuid>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub data: Vec<AuditLog>,
    pub page: i32,
    pub page_size: i32,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingRequest {
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageDirectoryEntry {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageDirectoryBrowseResponse {
    pub current_path: String,
    pub parent_path: Option<String>,
    pub directories: Vec<StorageDirectoryEntry>,
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct InstallBootstrapRequest {
    pub admin_email: String,
    pub admin_password: String,
    pub favicon_data_url: Option<String>,
    pub config: UpdateAdminSettingsConfigRequest,
}

#[derive(Debug, Serialize)]
pub struct InstallBootstrapResponse {
    pub user: crate::models::UserResponse,
    pub favicon_configured: bool,
    pub config: AdminSettingsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
    pub editable: bool,
    pub sensitive: bool,
    pub masked: bool,
    pub requires_confirmation: bool,
}

#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub database: ComponentStatus,
    pub cache: ComponentStatus,
    pub storage: ComponentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<HealthMetrics>,
}

#[derive(Debug, Serialize)]
pub struct HealthMetrics {
    pub images_count: i64,
    pub users_count: i64,
    pub storage_used_mb: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComponentStatus {
    pub status: String,
    pub message: Option<String>,
}

impl ComponentStatus {
    pub fn healthy() -> Self {
        Self {
            status: "healthy".to_string(),
            message: None,
        }
    }

    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            status: "unhealthy".to_string(),
            message: Some(message.into()),
        }
    }

    pub fn degraded(message: impl Into<String>) -> Self {
        Self {
            status: "degraded".to_string(),
            message: Some(message.into()),
        }
    }

    pub fn disabled(message: impl Into<String>) -> Self {
        Self {
            status: "disabled".to_string(),
            message: Some(message.into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SystemStats {
    pub total_users: i64,
    pub total_images: i64,
    pub total_storage: i64,
    pub total_views: i64,
    pub images_last_24h: i64,
    pub images_last_7d: i64,
}

impl axum::response::IntoResponse for SystemStats {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        axum::Json(self).into_response()
    }
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

impl Default for BackupSemantics {
    fn default() -> Self {
        Self::unknown()
    }
}

impl BackupSemantics {
    pub fn sqlite_database_snapshot() -> Self {
        Self {
            database_family: DatabaseKind::Sqlite.as_str().to_string(),
            backup_kind: "sqlite-database-snapshot".to_string(),
            backup_scope: "database-only".to_string(),
            restore_mode: "ui-restart-file-swap".to_string(),
            artifact_layout: "single-file-plus-manifest".to_string(),
            ui_restore_supported: true,
        }
    }

    pub fn mysql_logical_dump() -> Self {
        Self {
            database_family: DatabaseKind::MySql.as_str().to_string(),
            backup_kind: "mysql-logical-dump".to_string(),
            backup_scope: "database-only".to_string(),
            restore_mode: "ops-tooling-only".to_string(),
            artifact_layout: "single-file-plus-manifest".to_string(),
            ui_restore_supported: false,
        }
    }

    pub fn postgresql_logical_dump() -> Self {
        Self {
            database_family: DatabaseKind::Postgres.as_str().to_string(),
            backup_kind: "postgresql-logical-dump".to_string(),
            backup_scope: "database-only".to_string(),
            restore_mode: "download-only".to_string(),
            artifact_layout: "single-file-plus-manifest".to_string(),
            ui_restore_supported: false,
        }
    }

    pub fn unknown() -> Self {
        Self {
            database_family: "unknown".to_string(),
            backup_kind: "unknown".to_string(),
            backup_scope: "unknown".to_string(),
            restore_mode: "unknown".to_string(),
            artifact_layout: "unknown".to_string(),
            ui_restore_supported: false,
        }
    }

    pub fn from_database_kind(kind: DatabaseKind) -> Self {
        match kind {
            DatabaseKind::Postgres => Self::postgresql_logical_dump(),
            DatabaseKind::MySql => Self::mysql_logical_dump(),
            DatabaseKind::Sqlite => Self::sqlite_database_snapshot(),
        }
    }

    pub fn infer(filename: &str, database_kind: Option<DatabaseKind>) -> Self {
        if let Some(kind) = database_kind {
            return Self::from_database_kind(kind);
        }

        if filename.ends_with(".sqlite3") {
            Self::sqlite_database_snapshot()
        } else if filename.ends_with(".mysql.sql") {
            Self::mysql_logical_dump()
        } else if filename.ends_with(".sql") {
            Self::postgresql_logical_dump()
        } else {
            Self::unknown()
        }
    }

    pub fn infer_from_kind_str(filename: &str, database_kind: Option<&str>) -> Self {
        let parsed = database_kind.and_then(|value| DatabaseKind::parse(value).ok());
        Self::infer(filename, parsed)
    }

    pub fn is_unknown(&self) -> bool {
        self.backup_kind.eq_ignore_ascii_case("unknown")
    }
}

#[derive(Debug, Serialize)]
pub struct BackupResponse {
    pub filename: String,
    pub created_at: DateTime<Utc>,
    pub semantics: BackupSemantics,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupFileSummary {
    pub filename: String,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
    pub semantics: BackupSemantics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRestoreStorageSummary {
    pub storage_backend: String,
    pub local_storage_path: String,
    pub s3_endpoint: Option<String>,
    pub s3_region: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_prefix: Option<String>,
    pub s3_force_path_style: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadataManifest {
    #[serde(default = "default_backup_manifest_format_version")]
    pub format_version: u32,
    pub filename: String,
    pub created_at: DateTime<Utc>,
    pub database_kind: String,
    #[serde(default)]
    pub semantics: BackupSemantics,
    pub app_installed: bool,
    pub has_admin: bool,
    pub storage_signature: String,
    pub storage: BackupRestoreStorageSummary,
    pub object_rollback_anchor: BackupObjectRollbackAnchor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

fn default_restore_database_kind() -> String {
    "sqlite".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingBackupRestore {
    pub filename: String,
    #[serde(default = "default_restore_database_kind")]
    pub database_kind: String,
    #[serde(default)]
    pub semantics: BackupSemantics,
    pub requested_by_user_id: Uuid,
    pub requested_by_email: String,
    pub scheduled_at: DateTime<Utc>,
    pub backup_created_at: DateTime<Utc>,
    pub backup_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRestoreResult {
    pub status: String,
    pub filename: String,
    #[serde(default = "default_restore_database_kind")]
    pub database_kind: String,
    #[serde(default)]
    pub semantics: BackupSemantics,
    pub message: String,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: DateTime<Utc>,
    pub rollback_filename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRestoreStatusResponse {
    pub pending: Option<PendingBackupRestore>,
    pub last_result: Option<BackupRestoreResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRestoreScheduleResponse {
    pub scheduled: bool,
    pub restart_required: bool,
    pub pending: PendingBackupRestore,
    pub precheck: BackupRestorePrecheckResponse,
}

fn default_backup_manifest_format_version() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use crate::config::DatabaseKind;

    use super::BackupSemantics;

    #[test]
    fn backup_semantics_infer_from_database_kind() {
        let semantics = BackupSemantics::from_database_kind(DatabaseKind::Sqlite);
        assert_eq!(semantics.database_family, "sqlite");
        assert_eq!(semantics.backup_kind, "sqlite-database-snapshot");
        assert_eq!(semantics.restore_mode, "ui-restart-file-swap");
        assert!(semantics.ui_restore_supported);
    }

    #[test]
    fn backup_semantics_infer_from_filename_for_legacy_records() {
        let semantics = BackupSemantics::infer("backup_123.mysql.sql", None);
        assert_eq!(semantics.database_family, "mysql");
        assert_eq!(semantics.backup_kind, "mysql-logical-dump");
        assert_eq!(semantics.restore_mode, "ops-tooling-only");
        assert!(!semantics.ui_restore_supported);
    }
}
