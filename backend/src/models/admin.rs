use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
pub use shared_types::admin::{
    AdminSettingsConfig, AuditLog, AuditLogResponse, BackgroundTaskMetrics, ComponentStatus,
    HealthMetrics, HealthStatus, InstallBootstrapRequest, InstallBootstrapResponse,
    InstallStatusResponse, RuntimeBacklogMetrics, RuntimeObservabilitySnapshot,
    RuntimeOperationMetrics, Setting, StorageDirectoryBrowseResponse, StorageDirectoryEntry,
    SystemStats, UpdateAdminSettingsConfigRequest, UpdateSettingRequest,
};
pub use shared_types::backup::{
    BackupDatabaseFamily, BackupFileSummary, BackupObjectRollbackAnchor,
    BackupObjectRollbackStrategy, BackupResponse, BackupRestoreStorageSummary, BackupSemantics,
};
pub use shared_types::common::{HealthState, StorageBackendKind};
use uuid::Uuid;

use crate::config::DatabaseKind;
use crate::runtime_settings::StorageBackend as RuntimeStorageBackend;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AuditLogRecord {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<Uuid>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<AuditLogRecord> for AuditLog {
    fn from(value: AuditLogRecord) -> Self {
        Self {
            id: value.id.to_string(),
            user_id: value.user_id.map(|id| id.to_string()),
            action: value.action,
            target_type: value.target_type,
            target_id: value.target_id.map(|id| id.to_string()),
            details: value.details,
            ip_address: value.ip_address,
            created_at: value.created_at,
        }
    }
}

pub fn storage_backend_kind_from_runtime(value: RuntimeStorageBackend) -> StorageBackendKind {
    match value {
        RuntimeStorageBackend::Local => StorageBackendKind::Local,
    }
}

pub fn runtime_storage_backend_from_kind(
    value: StorageBackendKind,
) -> Option<RuntimeStorageBackend> {
    match value {
        StorageBackendKind::Local => Some(RuntimeStorageBackend::Local),
        StorageBackendKind::Unknown => None,
    }
}

pub fn backup_database_family_from_config(value: DatabaseKind) -> BackupDatabaseFamily {
    match value {
        DatabaseKind::Postgres => BackupDatabaseFamily::Postgres,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn config_database_kind_from_backup_family(
    value: BackupDatabaseFamily,
) -> Option<DatabaseKind> {
    match value {
        BackupDatabaseFamily::Postgres => Some(DatabaseKind::Postgres),
        _ => None,
    }
}

pub fn backup_semantics_from_database_kind(kind: DatabaseKind) -> BackupSemantics {
    match kind {
        DatabaseKind::Postgres => BackupSemantics::postgresql_logical_dump(),
    }
}

pub fn infer_backup_semantics(
    filename: &str,
    database_kind: Option<DatabaseKind>,
) -> BackupSemantics {
    if let Some(kind) = database_kind {
        return backup_semantics_from_database_kind(kind);
    }

    if filename.ends_with(".sql") {
        BackupSemantics::postgresql_logical_dump()
    } else {
        BackupSemantics::unknown()
    }
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

fn default_backup_manifest_format_version() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use crate::config::DatabaseKind;
    use shared_types::backup::{BackupKind, BackupRestoreStatus, RestoreMode};

    use super::{
        BackupDatabaseFamily, BackupObjectRollbackStrategy, HealthState, StorageBackendKind,
        backup_database_family_from_config, backup_semantics_from_database_kind,
        config_database_kind_from_backup_family, infer_backup_semantics,
    };

    #[test]
    fn backup_semantics_infer_from_database_kind() {
        let semantics = backup_semantics_from_database_kind(DatabaseKind::Postgres);
        assert_eq!(semantics.database_family, BackupDatabaseFamily::Postgres);
        assert_eq!(semantics.backup_kind, BackupKind::PostgresqlLogicalDump);
        assert_eq!(semantics.restore_mode, RestoreMode::DownloadOnly);
        assert!(!semantics.ui_restore_supported);
    }

    #[test]
    fn backup_semantics_infer_from_filename_for_legacy_records() {
        let semantics = infer_backup_semantics("backup_123.sql", None);
        assert_eq!(semantics.database_family, BackupDatabaseFamily::Postgres);
        assert_eq!(semantics.backup_kind, BackupKind::PostgresqlLogicalDump);
        assert_eq!(semantics.restore_mode, RestoreMode::DownloadOnly);
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
            BackupObjectRollbackStrategy::Unknown
        );
        assert_eq!(
            BackupObjectRollbackStrategy::parse("other"),
            BackupObjectRollbackStrategy::Unknown
        );
    }

    #[test]
    fn backup_database_family_round_trips_config_kind() {
        let family = backup_database_family_from_config(DatabaseKind::Postgres);

        assert_eq!(family, BackupDatabaseFamily::Postgres);
        assert_eq!(
            config_database_kind_from_backup_family(family),
            Some(DatabaseKind::Postgres)
        );
        assert_eq!(
            config_database_kind_from_backup_family(BackupDatabaseFamily::Unknown),
            None
        );
    }
}
