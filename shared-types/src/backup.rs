use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::StorageBackendKind;

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

    pub fn label(self, family: BackupDatabaseFamily) -> &'static str {
        match self {
            Self::SqliteDatabaseSnapshot => "SQLite 数据库快照",
            Self::MySqlLogicalDump => "MySQL / MariaDB 逻辑导出",
            Self::PostgresqlLogicalDump => "PostgreSQL 逻辑导出",
            Self::Unknown => family.label(),
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

    pub fn label(self) -> &'static str {
        match self {
            Self::UiRestartFileSwap => "重启前文件替换恢复",
            Self::UiRestartSqlImport => "重启前导入恢复",
            Self::OpsToolingOnly => "仅运维脚本恢复",
            Self::DownloadOnly => "仅下载，不支持页面恢复",
            Self::Unknown => "恢复方式未知",
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

    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "待执行",
            Self::Started => "执行中",
            Self::Completed => "已完成",
            Self::RolledBack => "已回滚",
            Self::Failed => "失败",
            Self::Unknown => "未知",
        }
    }

    pub fn surface_class(self) -> &'static str {
        match self {
            Self::Pending | Self::Started => "is-warning",
            Self::Completed => "is-info",
            Self::RolledBack | Self::Failed => "is-danger",
            Self::Unknown => "",
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

    pub fn kind_label(&self) -> &'static str {
        self.backup_kind.label(self.database_family)
    }

    pub fn restore_mode_label(&self) -> &'static str {
        self.restore_mode.label()
    }

    pub fn supports_restore(&self) -> bool {
        self.ui_restore_supported
    }

    pub fn is_sqlite_database_snapshot(&self) -> bool {
        self.backup_kind == BackupKind::SqliteDatabaseSnapshot
    }

    pub fn is_unknown(&self) -> bool {
        self.backup_kind == BackupKind::Unknown
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingBackupRestore {
    pub filename: String,
    #[serde(default = "default_restore_database_kind")]
    pub database_kind: BackupDatabaseFamily,
    #[serde(default)]
    pub semantics: BackupSemantics,
    pub requested_by_user_id: String,
    pub requested_by_email: String,
    pub scheduled_at: DateTime<Utc>,
    pub backup_created_at: DateTime<Utc>,
    pub backup_size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
