use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::DatabaseKind;
use crate::runtime_settings::StorageBackend as RuntimeStorageBackend;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum StorageBackendKind {
    Local,
    S3,
    Unknown,
}

impl StorageBackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::S3 => "s3",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "local" => Self::Local,
            "s3" => Self::S3,
            _ => Self::Unknown,
        }
    }

    pub fn from_runtime(value: RuntimeStorageBackend) -> Self {
        match value {
            RuntimeStorageBackend::Local => Self::Local,
            RuntimeStorageBackend::S3 => Self::S3,
        }
    }

    pub fn to_runtime(self) -> Option<RuntimeStorageBackend> {
        match self {
            Self::Local => Some(RuntimeStorageBackend::Local),
            Self::S3 => Some(RuntimeStorageBackend::S3),
            Self::Unknown => None,
        }
    }

    pub fn is_s3(self) -> bool {
        matches!(self, Self::S3)
    }
}

impl From<String> for StorageBackendKind {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<StorageBackendKind> for String {
    fn from(value: StorageBackendKind) -> Self {
        value.as_str().to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
    Disabled,
    Bootstrapping,
    Unknown,
}

impl HealthState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
            Self::Disabled => "disabled",
            Self::Bootstrapping => "bootstrapping",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "healthy" => Self::Healthy,
            "degraded" => Self::Degraded,
            "unhealthy" => Self::Unhealthy,
            "disabled" => Self::Disabled,
            "bootstrapping" => Self::Bootstrapping,
            _ => Self::Unknown,
        }
    }
}

impl From<String> for HealthState {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<HealthState> for String {
    fn from(value: HealthState) -> Self {
        value.as_str().to_string()
    }
}

#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: HealthState,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum BackupDatabaseFamily {
    Postgres,
    MySql,
    Sqlite,
    Unknown,
}

impl BackupDatabaseFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Postgres => "postgresql",
            Self::MySql => "mysql",
            Self::Sqlite => "sqlite",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "postgresql" | "postgres" => Self::Postgres,
            "mysql" | "mariadb" => Self::MySql,
            "sqlite" => Self::Sqlite,
            _ => Self::Unknown,
        }
    }

    pub fn from_database_kind(kind: DatabaseKind) -> Self {
        match kind {
            DatabaseKind::Postgres => Self::Postgres,
            DatabaseKind::MySql => Self::MySql,
            DatabaseKind::Sqlite => Self::Sqlite,
        }
    }

    pub fn to_database_kind(self) -> Option<DatabaseKind> {
        match self {
            Self::Postgres => Some(DatabaseKind::Postgres),
            Self::MySql => Some(DatabaseKind::MySql),
            Self::Sqlite => Some(DatabaseKind::Sqlite),
            Self::Unknown => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Sqlite => "SQLite",
            Self::MySql => "MySQL / MariaDB",
            Self::Postgres => "PostgreSQL",
            Self::Unknown => "数据库",
        }
    }
}

impl From<String> for BackupDatabaseFamily {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<BackupDatabaseFamily> for String {
    fn from(value: BackupDatabaseFamily) -> Self {
        value.as_str().to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum BackupRestoreStatus {
    Pending,
    Started,
    Completed,
    RolledBack,
    Failed,
    Unknown,
}

impl BackupRestoreStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Started => "started",
            Self::Completed => "completed",
            Self::RolledBack => "rolled_back",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "pending" => Self::Pending,
            "started" => Self::Started,
            "completed" => Self::Completed,
            "rolled_back" => Self::RolledBack,
            "failed" => Self::Failed,
            _ => Self::Unknown,
        }
    }
}

impl From<String> for BackupRestoreStatus {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<BackupRestoreStatus> for String {
    fn from(value: BackupRestoreStatus) -> Self {
        value.as_str().to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum BackupKind {
    SqliteDatabaseSnapshot,
    MySqlLogicalDump,
    PostgresqlLogicalDump,
    Unknown,
}

impl BackupKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SqliteDatabaseSnapshot => "sqlite-database-snapshot",
            Self::MySqlLogicalDump => "mysql-logical-dump",
            Self::PostgresqlLogicalDump => "postgresql-logical-dump",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "sqlite-database-snapshot" => Self::SqliteDatabaseSnapshot,
            "mysql-logical-dump" => Self::MySqlLogicalDump,
            "postgresql-logical-dump" => Self::PostgresqlLogicalDump,
            _ => Self::Unknown,
        }
    }
}

impl From<String> for BackupKind {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<BackupKind> for String {
    fn from(value: BackupKind) -> Self {
        value.as_str().to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum BackupScope {
    DatabaseOnly,
    Unknown,
}

impl BackupScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DatabaseOnly => "database-only",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "database-only" => Self::DatabaseOnly,
            _ => Self::Unknown,
        }
    }
}

impl From<String> for BackupScope {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<BackupScope> for String {
    fn from(value: BackupScope) -> Self {
        value.as_str().to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum RestoreMode {
    UiRestartFileSwap,
    UiRestartSqlImport,
    OpsToolingOnly,
    DownloadOnly,
    Unknown,
}

impl RestoreMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UiRestartFileSwap => "ui-restart-file-swap",
            Self::UiRestartSqlImport => "ui-restart-sql-import",
            Self::OpsToolingOnly => "ops-tooling-only",
            Self::DownloadOnly => "download-only",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "ui-restart-file-swap" => Self::UiRestartFileSwap,
            "ui-restart-sql-import" => Self::UiRestartSqlImport,
            "ops-tooling-only" => Self::OpsToolingOnly,
            "download-only" => Self::DownloadOnly,
            _ => Self::Unknown,
        }
    }
}

impl From<String> for RestoreMode {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<RestoreMode> for String {
    fn from(value: RestoreMode) -> Self {
        value.as_str().to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum ArtifactLayout {
    SingleFilePlusManifest,
    Unknown,
}

impl ArtifactLayout {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SingleFilePlusManifest => "single-file-plus-manifest",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "single-file-plus-manifest" => Self::SingleFilePlusManifest,
            _ => Self::Unknown,
        }
    }
}

impl From<String> for ArtifactLayout {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<ArtifactLayout> for String {
    fn from(value: ArtifactLayout) -> Self {
        value.as_str().to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupSemantics {
    pub database_family: BackupDatabaseFamily,
    pub backup_kind: BackupKind,
    pub backup_scope: BackupScope,
    pub restore_mode: RestoreMode,
    pub artifact_layout: ArtifactLayout,
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
            database_family: BackupDatabaseFamily::Sqlite,
            backup_kind: BackupKind::SqliteDatabaseSnapshot,
            backup_scope: BackupScope::DatabaseOnly,
            restore_mode: RestoreMode::UiRestartFileSwap,
            artifact_layout: ArtifactLayout::SingleFilePlusManifest,
            ui_restore_supported: true,
        }
    }

    pub fn mysql_logical_dump() -> Self {
        Self {
            database_family: BackupDatabaseFamily::MySql,
            backup_kind: BackupKind::MySqlLogicalDump,
            backup_scope: BackupScope::DatabaseOnly,
            restore_mode: RestoreMode::OpsToolingOnly,
            artifact_layout: ArtifactLayout::SingleFilePlusManifest,
            ui_restore_supported: false,
        }
    }

    pub fn postgresql_logical_dump() -> Self {
        Self {
            database_family: BackupDatabaseFamily::Postgres,
            backup_kind: BackupKind::PostgresqlLogicalDump,
            backup_scope: BackupScope::DatabaseOnly,
            restore_mode: RestoreMode::DownloadOnly,
            artifact_layout: ArtifactLayout::SingleFilePlusManifest,
            ui_restore_supported: false,
        }
    }

    pub fn unknown() -> Self {
        Self {
            database_family: BackupDatabaseFamily::Unknown,
            backup_kind: BackupKind::Unknown,
            backup_scope: BackupScope::Unknown,
            restore_mode: RestoreMode::Unknown,
            artifact_layout: ArtifactLayout::Unknown,
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

    pub fn is_unknown(&self) -> bool {
        self.backup_kind == BackupKind::Unknown
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
    pub storage_backend: StorageBackendKind,
    pub local_storage_path: String,
    pub s3_endpoint: Option<String>,
    pub s3_region: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_prefix: Option<String>,
    pub s3_force_path_style: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum BackupObjectRollbackStrategy {
    LocalDirectorySnapshot,
    S3VersionedRollbackAnchor,
    Unknown,
}

impl BackupObjectRollbackStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalDirectorySnapshot => "local-directory-snapshot",
            Self::S3VersionedRollbackAnchor => "s3-versioned-rollback-anchor",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "local-directory-snapshot" => Self::LocalDirectorySnapshot,
            "s3-versioned-rollback-anchor" => Self::S3VersionedRollbackAnchor,
            _ => Self::Unknown,
        }
    }

    pub fn is_local_directory_snapshot(self) -> bool {
        matches!(self, Self::LocalDirectorySnapshot)
    }

    pub fn is_s3_versioned_rollback_anchor(self) -> bool {
        matches!(self, Self::S3VersionedRollbackAnchor)
    }
}

impl From<String> for BackupObjectRollbackStrategy {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<BackupObjectRollbackStrategy> for String {
    fn from(value: BackupObjectRollbackStrategy) -> Self {
        value.as_str().to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupObjectRollbackAnchor {
    pub strategy: BackupObjectRollbackStrategy,
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
    pub database_kind: BackupDatabaseFamily,
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
    pub current_database_kind: BackupDatabaseFamily,
    pub backup_database_kind: BackupDatabaseFamily,
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

fn default_restore_database_kind() -> BackupDatabaseFamily {
    BackupDatabaseFamily::Sqlite
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingBackupRestore {
    pub filename: String,
    #[serde(default = "default_restore_database_kind")]
    pub database_kind: BackupDatabaseFamily,
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
    pub status: BackupRestoreStatus,
    pub filename: String,
    #[serde(default = "default_restore_database_kind")]
    pub database_kind: BackupDatabaseFamily,
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

    use super::{
        BackupDatabaseFamily, BackupKind, BackupObjectRollbackStrategy, BackupRestoreStatus,
        BackupSemantics, HealthState, RestoreMode, StorageBackendKind,
    };

    #[test]
    fn backup_semantics_infer_from_database_kind() {
        let semantics = BackupSemantics::from_database_kind(DatabaseKind::Sqlite);
        assert_eq!(semantics.database_family, BackupDatabaseFamily::Sqlite);
        assert_eq!(semantics.backup_kind, BackupKind::SqliteDatabaseSnapshot);
        assert_eq!(semantics.restore_mode, RestoreMode::UiRestartFileSwap);
        assert!(semantics.ui_restore_supported);
    }

    #[test]
    fn backup_semantics_infer_from_filename_for_legacy_records() {
        let semantics = BackupSemantics::infer("backup_123.mysql.sql", None);
        assert_eq!(semantics.database_family, BackupDatabaseFamily::MySql);
        assert_eq!(semantics.backup_kind, BackupKind::MySqlLogicalDump);
        assert_eq!(semantics.restore_mode, RestoreMode::OpsToolingOnly);
        assert!(!semantics.ui_restore_supported);
    }

    #[test]
    fn backup_restore_status_parses_legacy_string_values() {
        assert_eq!(
            BackupRestoreStatus::parse("rolled_back"),
            BackupRestoreStatus::RolledBack
        );
        assert_eq!(
            BackupRestoreStatus::parse("started"),
            BackupRestoreStatus::Started
        );
        assert_eq!(
            BackupRestoreStatus::parse("other"),
            BackupRestoreStatus::Unknown
        );
    }

    #[test]
    fn health_state_parses_legacy_string_values() {
        assert_eq!(HealthState::parse("healthy"), HealthState::Healthy);
        assert_eq!(HealthState::parse("DISABLED"), HealthState::Disabled);
        assert_eq!(
            HealthState::parse("bootstrapping"),
            HealthState::Bootstrapping
        );
        assert_eq!(HealthState::parse("other"), HealthState::Unknown);
    }

    #[test]
    fn storage_backend_kind_parses_legacy_string_values() {
        assert_eq!(
            StorageBackendKind::parse("local"),
            StorageBackendKind::Local
        );
        assert_eq!(StorageBackendKind::parse("S3"), StorageBackendKind::S3);
        assert_eq!(
            StorageBackendKind::parse("ftp"),
            StorageBackendKind::Unknown
        );
    }

    #[test]
    fn backup_object_rollback_strategy_parses_legacy_string_values() {
        assert_eq!(
            BackupObjectRollbackStrategy::parse("local-directory-snapshot"),
            BackupObjectRollbackStrategy::LocalDirectorySnapshot
        );
        assert_eq!(
            BackupObjectRollbackStrategy::parse("S3-versioned-rollback-anchor"),
            BackupObjectRollbackStrategy::S3VersionedRollbackAnchor
        );
        assert_eq!(
            BackupObjectRollbackStrategy::parse("other"),
            BackupObjectRollbackStrategy::Unknown
        );
    }
}
