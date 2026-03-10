use uuid::Uuid;

use super::AdminDomainService;
use crate::audit::log_audit_db;
use crate::error::AppError;
use crate::models::{
    BackupRestorePrecheckResponse, BackupRestoreScheduleResponse, BackupRestoreStatusResponse,
};

impl AdminDomainService {
    pub async fn get_restore_status(&self) -> Result<BackupRestoreStatusResponse, AppError> {
        crate::sqlite_restore::load_restore_status()
            .await
            .map_err(AppError::Internal)
    }

    pub async fn precheck_restore(
        &self,
        admin_user_id: Uuid,
        admin_email: &str,
        filename: &str,
    ) -> Result<BackupRestorePrecheckResponse, AppError> {
        let current_storage = self.storage_manager.active_settings().storage_settings();
        let response =
            crate::sqlite_restore::precheck_restore(&self.config, &current_storage, filename)
                .await?;

        let action = if response.eligible {
            "admin.maintenance.database_restore.prechecked"
        } else {
            "admin.maintenance.database_restore.precheck_failed"
        };
        log_audit_db(
            &self.database,
            Some(admin_user_id),
            action,
            "maintenance",
            None,
            None,
            Some(serde_json::json!({
                "admin_email": admin_email,
                "filename": response.filename,
                "eligible": response.eligible,
                "blockers": response.blockers,
                "warnings": response.warnings,
                "risk_level": "danger",
            })),
        )
        .await;

        Ok(response)
    }

    pub async fn schedule_restore(
        &self,
        admin_user_id: Uuid,
        admin_email: &str,
        filename: &str,
    ) -> Result<BackupRestoreScheduleResponse, AppError> {
        let current_storage = self.storage_manager.active_settings().storage_settings();
        let response = crate::sqlite_restore::schedule_restore(
            &self.config,
            &current_storage,
            admin_user_id,
            admin_email,
            filename,
        )
        .await?;

        log_audit_db(
            &self.database,
            Some(admin_user_id),
            "admin.maintenance.database_restore.scheduled",
            "maintenance",
            None,
            None,
            Some(serde_json::json!({
                "admin_email": admin_email,
                "filename": response.pending.filename,
                "scheduled_at": response.pending.scheduled_at,
                "backup_created_at": response.pending.backup_created_at,
                "backup_size_bytes": response.pending.backup_size_bytes,
                "restart_required": true,
                "risk_level": "danger",
            })),
        )
        .await;

        Ok(response)
    }
}
