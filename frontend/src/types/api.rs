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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum UserRole {
    Admin,
    User,
    Unknown,
}

impl UserRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::User => "user",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "admin" => Self::Admin,
            "user" => Self::User,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Admin => "管理员",
            Self::User => "普通用户",
            Self::Unknown => "未知角色",
        }
    }

    pub fn surface_class(self) -> &'static str {
        match self {
            Self::Admin => "is-admin",
            Self::User | Self::Unknown => "is-user",
        }
    }

    pub fn is_admin(self) -> bool {
        matches!(self, Self::Admin)
    }
}

impl From<String> for UserRole {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<UserRole> for String {
    fn from(value: UserRole) -> Self {
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

    pub fn label(self) -> &'static str {
        match self {
            Self::Healthy => "健康",
            Self::Degraded => "降级",
            Self::Unhealthy => "异常",
            Self::Disabled => "已禁用",
            Self::Bootstrapping => "引导中",
            Self::Unknown => "异常",
        }
    }

    pub fn surface_class(self) -> &'static str {
        match self {
            Self::Healthy | Self::Disabled => "is-healthy",
            Self::Degraded | Self::Unhealthy | Self::Bootstrapping | Self::Unknown => {
                "is-unhealthy"
            }
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

    pub fn label(self) -> &'static str {
        match self {
            Self::Local => "本地目录",
            Self::S3 => "对象存储",
            Self::Unknown => "未知后端",
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

/// 用户响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub email: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
}

/// 分页参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            page_size: Some(20),
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
}

/// 设置过期请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetExpiryRequest {
    pub expires_at: Option<DateTime<Utc>>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum BootstrapDatabaseKind {
    Postgres,
    MySql,
    Sqlite,
    Unknown,
}

impl BootstrapDatabaseKind {
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
            Self::Postgres => "PostgreSQL",
            Self::MySql => "MySQL / MariaDB",
            Self::Sqlite => "SQLite",
            Self::Unknown => "未识别数据库",
        }
    }
}

impl From<String> for BootstrapDatabaseKind {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<BootstrapDatabaseKind> for String {
    fn from(value: BootstrapDatabaseKind) -> Self {
        value.as_str().to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BootstrapStatusResponse {
    pub mode: String,
    pub database_kind: BootstrapDatabaseKind,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBootstrapDatabaseConfigRequest {
    pub database_kind: BootstrapDatabaseKind,
    pub database_url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateBootstrapDatabaseConfigResponse {
    pub database_kind: BootstrapDatabaseKind,
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
    pub status: HealthState,
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
    pub status: HealthState,
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

impl BackupSemantics {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingBackupRestore {
    pub filename: String,
    pub database_kind: BackupDatabaseFamily,
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
    pub database_kind: BackupDatabaseFamily,
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
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUpdateRequest {
    pub role: Option<UserRole>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_semantics_deserializes_legacy_string_payloads() {
        let semantics: BackupSemantics = serde_json::from_value(serde_json::json!({
            "database_family": "sqlite",
            "backup_kind": "sqlite-database-snapshot",
            "backup_scope": "database-only",
            "restore_mode": "ui-restart-file-swap",
            "artifact_layout": "single-file-plus-manifest",
            "ui_restore_supported": true
        }))
        .expect("legacy string payload should deserialize");

        assert_eq!(semantics.database_family, BackupDatabaseFamily::Sqlite);
        assert_eq!(semantics.backup_kind, BackupKind::SqliteDatabaseSnapshot);
        assert_eq!(semantics.backup_scope, BackupScope::DatabaseOnly);
        assert_eq!(semantics.restore_mode, RestoreMode::UiRestartFileSwap);
        assert_eq!(
            semantics.artifact_layout,
            ArtifactLayout::SingleFilePlusManifest
        );
        assert!(semantics.supports_restore());
        assert!(semantics.is_sqlite_database_snapshot());
        assert_eq!(semantics.kind_label(), "SQLite 数据库快照");
        assert_eq!(semantics.restore_mode_label(), "重启前文件替换恢复");
    }

    #[test]
    fn backup_semantics_unknown_values_fall_back_safely() {
        let semantics: BackupSemantics = serde_json::from_value(serde_json::json!({
            "database_family": "oracle",
            "backup_kind": "physical-copy",
            "backup_scope": "cluster",
            "restore_mode": "manual",
            "artifact_layout": "tarball",
            "ui_restore_supported": false
        }))
        .expect("unknown payload should still deserialize");

        assert_eq!(semantics.database_family, BackupDatabaseFamily::Unknown);
        assert_eq!(semantics.backup_kind, BackupKind::Unknown);
        assert_eq!(semantics.backup_scope, BackupScope::Unknown);
        assert_eq!(semantics.restore_mode, RestoreMode::Unknown);
        assert_eq!(semantics.artifact_layout, ArtifactLayout::Unknown);
        assert!(!semantics.supports_restore());
        assert!(!semantics.is_sqlite_database_snapshot());
        assert_eq!(semantics.kind_label(), "数据库");
        assert_eq!(semantics.restore_mode_label(), "恢复方式未知");
    }

    #[test]
    fn restore_result_deserializes_legacy_string_enums() {
        let result: BackupRestoreResult = serde_json::from_value(serde_json::json!({
            "status": "started",
            "filename": "backup_demo.sqlite3",
            "database_kind": "sqlite",
            "semantics": {
                "database_family": "sqlite",
                "backup_kind": "sqlite-database-snapshot",
                "backup_scope": "database-only",
                "restore_mode": "ui-restart-file-swap",
                "artifact_layout": "single-file-plus-manifest",
                "ui_restore_supported": true
            },
            "message": "running",
            "scheduled_at": "2026-03-15T00:00:00Z",
            "started_at": "2026-03-15T00:01:00Z",
            "finished_at": "2026-03-15T00:02:00Z",
            "rollback_filename": null
        }))
        .expect("legacy restore result should deserialize");

        assert_eq!(result.status, BackupRestoreStatus::Started);
        assert_eq!(result.status.label(), "执行中");
        assert_eq!(result.status.surface_class(), "is-warning");
        assert_eq!(result.database_kind, BackupDatabaseFamily::Sqlite);
        assert_eq!(result.database_kind.label(), "SQLite");
    }

    #[test]
    fn bootstrap_status_deserializes_database_kind_enum() {
        let status: BootstrapStatusResponse = serde_json::from_value(serde_json::json!({
            "mode": "bootstrap",
            "database_kind": "mysql",
            "database_configured": true,
            "database_url_masked": "mysql://******",
            "cache_configured": false,
            "cache_url_masked": null,
            "restart_required": true,
            "runtime_error": null
        }))
        .expect("bootstrap status should deserialize");

        assert_eq!(status.database_kind, BootstrapDatabaseKind::MySql);
        assert_eq!(status.database_kind.label(), "MySQL / MariaDB");
    }

    #[test]
    fn backup_object_rollback_anchor_deserializes_strategy_enum() {
        let anchor: BackupObjectRollbackAnchor = serde_json::from_value(serde_json::json!({
            "strategy": "local-directory-snapshot",
            "checkpoint_at": "2026-03-15T00:00:00Z",
            "local_storage_path": "/data/images",
            "s3_endpoint": null,
            "s3_region": null,
            "s3_bucket": null,
            "s3_prefix": null,
            "s3_force_path_style": true,
            "s3_bucket_versioning_status": null,
            "capture_error": null
        }))
        .expect("rollback anchor should deserialize");

        assert_eq!(
            anchor.strategy,
            BackupObjectRollbackStrategy::LocalDirectorySnapshot
        );
        assert!(anchor.strategy.is_local_directory_snapshot());
    }

    #[test]
    fn backup_object_rollback_strategy_unknown_values_fall_back_safely() {
        let strategy = BackupObjectRollbackStrategy::parse("snapshot-copy");

        assert_eq!(strategy, BackupObjectRollbackStrategy::Unknown);
        assert!(!strategy.is_local_directory_snapshot());
        assert!(!strategy.is_s3_versioned_rollback_anchor());
    }

    #[test]
    fn user_response_deserializes_legacy_role_string_enum() {
        let user: UserResponse = serde_json::from_value(serde_json::json!({
            "email": "admin@example.com",
            "role": "admin",
            "created_at": "2026-03-15T00:00:00Z"
        }))
        .expect("legacy user response should deserialize");

        assert_eq!(user.role, UserRole::Admin);
        assert!(user.role.is_admin());
        assert_eq!(user.role.label(), "管理员");
        assert_eq!(user.role.surface_class(), "is-admin");
    }

    #[test]
    fn user_role_unknown_values_fall_back_safely() {
        let role = UserRole::parse("moderator");

        assert_eq!(role, UserRole::Unknown);
        assert!(!role.is_admin());
        assert_eq!(role.label(), "未知角色");
        assert_eq!(role.surface_class(), "is-user");
    }

    #[test]
    fn health_status_deserializes_legacy_string_enums() {
        let health: HealthStatus = serde_json::from_value(serde_json::json!({
            "status": "degraded",
            "timestamp": "2026-03-15T00:00:00Z",
            "database": { "status": "healthy", "message": null },
            "cache": { "status": "disabled", "message": "cache off" },
            "storage": { "status": "unhealthy", "message": "disk error" },
            "version": "0.1.0",
            "uptime_seconds": 60,
            "metrics": null
        }))
        .expect("legacy health status should deserialize");

        assert_eq!(health.status, HealthState::Degraded);
        assert_eq!(health.status.label(), "降级");
        assert_eq!(health.database.status, HealthState::Healthy);
        assert_eq!(health.cache.status, HealthState::Disabled);
        assert_eq!(health.storage.status, HealthState::Unhealthy);
    }

    #[test]
    fn health_state_unknown_values_fall_back_safely() {
        let state = HealthState::parse("pending");

        assert_eq!(state, HealthState::Unknown);
        assert_eq!(state.label(), "异常");
        assert_eq!(state.surface_class(), "is-unhealthy");
    }

    #[test]
    fn admin_settings_deserializes_storage_backend_enum() {
        let config: AdminSettingsConfig = serde_json::from_value(serde_json::json!({
            "site_name": "Vansour Image",
            "storage_backend": "s3",
            "local_storage_path": "/data/images",
            "mail_enabled": false,
            "mail_smtp_host": "",
            "mail_smtp_port": 587,
            "mail_smtp_user": null,
            "mail_smtp_password_set": false,
            "mail_from_email": "",
            "mail_from_name": "",
            "mail_link_base_url": "",
            "s3_endpoint": null,
            "s3_region": null,
            "s3_bucket": null,
            "s3_prefix": null,
            "s3_access_key": null,
            "s3_secret_key_set": false,
            "s3_force_path_style": true,
            "restart_required": false
        }))
        .expect("legacy storage backend should deserialize");

        assert_eq!(config.storage_backend, StorageBackendKind::S3);
        assert!(config.storage_backend.is_s3());
        assert_eq!(config.storage_backend.label(), "对象存储");
    }

    #[test]
    fn storage_backend_unknown_values_fall_back_safely() {
        let backend = StorageBackendKind::parse("ftp");

        assert_eq!(backend, StorageBackendKind::Unknown);
        assert!(!backend.is_s3());
        assert_eq!(backend.label(), "未知后端");
    }
}
