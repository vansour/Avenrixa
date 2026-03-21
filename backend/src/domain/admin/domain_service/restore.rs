use uuid::Uuid;

use super::AdminDomainService;
use crate::error::AppError;
use crate::models::{
    BackupRestorePrecheckResponse, BackupRestoreScheduleResponse, BackupRestoreStatusResponse,
};

impl AdminDomainService {
    pub async fn get_restore_status(&self) -> Result<BackupRestoreStatusResponse, AppError> {
        Ok(BackupRestoreStatusResponse {
            pending: None,
            last_result: None,
        })
    }

    pub async fn precheck_restore(
        &self,
        _admin_user_id: Uuid,
        _admin_email: &str,
        _filename: &str,
    ) -> Result<BackupRestorePrecheckResponse, AppError> {
        Err(AppError::ValidationError(
            "页面恢复功能已移除；请改为下载备份后使用运维脚本恢复。".to_string(),
        ))
    }

    pub async fn schedule_restore(
        &self,
        _admin_user_id: Uuid,
        _admin_email: &str,
        _filename: &str,
    ) -> Result<BackupRestoreScheduleResponse, AppError> {
        Err(AppError::ValidationError(
            "页面恢复功能已移除；请改为下载备份后使用运维脚本恢复。".to_string(),
        ))
    }
}
