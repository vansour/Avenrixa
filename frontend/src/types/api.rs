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
