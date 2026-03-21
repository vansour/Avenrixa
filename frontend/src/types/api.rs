pub use shared_types::admin::{
    AdminSettingsConfig, AdminUserSummary, AuditLog, AuditLogResponse, ComponentStatus,
    HealthMetrics, HealthStatus, InstallBootstrapRequest, InstallBootstrapResponse,
    InstallStatusResponse, Setting, StorageDirectoryBrowseResponse, StorageDirectoryEntry,
    SystemStats, UpdateAdminSettingsConfigRequest, UpdateSettingRequest,
};
pub use shared_types::auth::{
    EmailVerificationConfirmRequest, LoginRequest, PasswordResetConfirmRequest,
    PasswordResetRequest, RegisterRequest, UpdateProfileRequest, UserResponse, UserUpdateRequest,
};
pub use shared_types::backup::{
    ArtifactLayout, BackupDatabaseFamily, BackupFileSummary, BackupKind,
    BackupObjectRollbackAnchor, BackupObjectRollbackStrategy, BackupResponse,
    BackupRestorePrecheckResponse, BackupRestoreResult, BackupRestoreScheduleResponse,
    BackupRestoreStatus, BackupRestoreStatusResponse, BackupRestoreStorageSummary, BackupScope,
    BackupSemantics, PendingBackupRestore, RestoreMode,
};
pub use shared_types::bootstrap::{
    BootstrapDatabaseKind, BootstrapStatusResponse, UpdateBootstrapDatabaseConfigRequest,
    UpdateBootstrapDatabaseConfigResponse,
};
pub use shared_types::common::{HealthState, StorageBackendKind, UserRole};
pub use shared_types::image::{DeleteRequest, ImageResponse, SetExpiryRequest};
pub use shared_types::pagination::{CursorPaginated, CursorPaginationParams, PaginationParams};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_semantics_deserializes_legacy_string_payloads() {
        let semantics: BackupSemantics = serde_json::from_value(serde_json::json!({
            "database_family": "mysql",
            "backup_kind": "mysql-logical-dump",
            "backup_scope": "database-only",
            "restore_mode": "ops-tooling-only",
            "artifact_layout": "single-file-plus-manifest",
            "ui_restore_supported": false
        }))
        .expect("legacy string payload should deserialize");

        assert_eq!(semantics.database_family, BackupDatabaseFamily::MySql);
        assert_eq!(semantics.backup_kind, BackupKind::MySqlLogicalDump);
        assert_eq!(semantics.backup_scope, BackupScope::DatabaseOnly);
        assert_eq!(semantics.restore_mode, RestoreMode::OpsToolingOnly);
        assert_eq!(
            semantics.artifact_layout,
            ArtifactLayout::SingleFilePlusManifest
        );
        assert!(!semantics.supports_restore());
        assert_eq!(semantics.kind_label(), "MySQL / MariaDB 逻辑导出");
        assert_eq!(semantics.restore_mode_label(), "仅运维脚本恢复");
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
        assert_eq!(semantics.kind_label(), "数据库");
        assert_eq!(semantics.restore_mode_label(), "恢复方式未知");
    }

    #[test]
    fn restore_result_deserializes_legacy_string_enums() {
        let result: BackupRestoreResult = serde_json::from_value(serde_json::json!({
            "status": "started",
            "filename": "backup_demo.mysql.sql",
            "database_kind": "mysql",
            "semantics": {
                "database_family": "mysql",
                "backup_kind": "mysql-logical-dump",
                "backup_scope": "database-only",
                "restore_mode": "ops-tooling-only",
                "artifact_layout": "single-file-plus-manifest",
                "ui_restore_supported": false
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
        assert_eq!(result.database_kind, BackupDatabaseFamily::MySql);
        assert_eq!(result.database_kind.label(), "MySQL / MariaDB");
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
            "version": "0.1.2-rc.1",
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
            "site_name": "Avenrixa",
            "storage_backend": "local",
            "local_storage_path": "/data/images",
            "mail_enabled": false,
            "mail_smtp_host": "",
            "mail_smtp_port": 587,
            "mail_smtp_user": null,
            "mail_smtp_password_set": false,
            "mail_from_email": "",
            "mail_from_name": "",
            "mail_link_base_url": "",
            "restart_required": false
        }))
        .expect("local storage backend should deserialize");

        assert_eq!(config.storage_backend, StorageBackendKind::Local);
        assert_eq!(config.storage_backend.label(), "本地目录");
    }

    #[test]
    fn storage_backend_unknown_values_fall_back_safely() {
        let backend = StorageBackendKind::parse("ftp");

        assert_eq!(backend, StorageBackendKind::Unknown);
        assert_eq!(backend.label(), "未知后端");
    }
}
