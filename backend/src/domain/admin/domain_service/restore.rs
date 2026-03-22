use uuid::Uuid;

use super::AdminDomainService;
use crate::audit::log_audit_db;
use crate::db::DatabasePool;
use crate::error::AppError;
use crate::models::{
    BackupRestorePrecheckResponse, BackupRestoreScheduleResponse, BackupRestoreStatusResponse,
    backup_semantics_from_database_kind,
};

const PAGE_RESTORE_REMOVED_MESSAGE: &str = "页面恢复功能已移除；请改为下载备份后使用运维脚本恢复。";

fn spawn_restore_audit(
    database: DatabasePool,
    admin_user_id: Uuid,
    action: &'static str,
    details: serde_json::Value,
) {
    tokio::spawn(async move {
        log_audit_db(
            &database,
            Some(admin_user_id),
            action,
            "maintenance",
            None,
            None,
            Some(details),
        )
        .await;
    });
}

impl AdminDomainService {
    fn restore_rejection_details(&self, admin_email: &str, filename: &str) -> serde_json::Value {
        let semantics = backup_semantics_from_database_kind(self.config.database.kind);

        serde_json::json!({
            "admin_email": admin_email,
            "filename": filename,
            "result": "rejected",
            "risk_level": "info",
            "database_kind": semantics.database_family,
            "backup_kind": semantics.backup_kind,
            "backup_scope": semantics.backup_scope,
            "restore_mode": semantics.restore_mode,
            "ui_restore_supported": semantics.ui_restore_supported,
            "reason": "ui_restore_removed",
            "message": PAGE_RESTORE_REMOVED_MESSAGE,
        })
    }

    pub async fn get_restore_status(&self) -> Result<BackupRestoreStatusResponse, AppError> {
        Ok(BackupRestoreStatusResponse {
            pending: None,
            last_result: None,
        })
    }

    pub async fn precheck_restore(
        &self,
        admin_user_id: Uuid,
        admin_email: &str,
        filename: &str,
    ) -> Result<BackupRestorePrecheckResponse, AppError> {
        spawn_restore_audit(
            self.database.clone(),
            admin_user_id,
            "admin.maintenance.database_restore.precheck_failed",
            self.restore_rejection_details(admin_email, filename),
        );

        Err(AppError::ValidationError(
            PAGE_RESTORE_REMOVED_MESSAGE.to_string(),
        ))
    }

    pub async fn schedule_restore(
        &self,
        admin_user_id: Uuid,
        admin_email: &str,
        filename: &str,
    ) -> Result<BackupRestoreScheduleResponse, AppError> {
        spawn_restore_audit(
            self.database.clone(),
            admin_user_id,
            "admin.maintenance.database_restore.schedule_failed",
            self.restore_rejection_details(admin_email, filename),
        );

        Err(AppError::ValidationError(
            PAGE_RESTORE_REMOVED_MESSAGE.to_string(),
        ))
    }
}
