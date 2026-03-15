use crate::services::AdminService;
use crate::types::api::{BackupFileSummary, BackupRestoreStatusResponse};
use dioxus::prelude::*;

use super::super::merge_messages;

pub(super) fn build_backup_downloads(
    admin_service: &AdminService,
    backup_files: Signal<Vec<BackupFileSummary>>,
) -> Vec<(BackupFileSummary, String)> {
    backup_files()
        .into_iter()
        .map(|backup| {
            let download_url = admin_service.backup_download_url(&backup.filename);
            (backup, download_url)
        })
        .collect()
}

pub(super) fn combined_error_message(
    error_message: Signal<String>,
    backup_list_error_message: Signal<String>,
    restore_status_error_message: Signal<String>,
) -> String {
    let combined = merge_messages(&error_message(), &backup_list_error_message());
    merge_messages(&combined, &restore_status_error_message())
}

pub(super) fn is_maintenance_busy(
    is_cleaning_expired: Signal<bool>,
    is_backing_up: Signal<bool>,
    deleting_backup_filename: Signal<Option<String>>,
    processing_restore_filename: Signal<Option<String>>,
) -> bool {
    is_cleaning_expired()
        || is_backing_up()
        || deleting_backup_filename().is_some()
        || processing_restore_filename().is_some()
}

pub(super) fn restore_blocking_message(
    backup: &BackupFileSummary,
    restore_status: Option<&BackupRestoreStatusResponse>,
) -> Option<String> {
    if !backup.semantics.supports_restore() {
        return Some("当前这类备份不支持页面恢复，请改为下载备份后走运维侧恢复。".to_string());
    }

    let pending = restore_status.and_then(|status| status.pending.as_ref())?;
    Some(if pending.filename == backup.filename {
        format!(
            "备份 {} 已经写入恢复计划，请立即重启服务。",
            backup.filename
        )
    } else {
        format!(
            "当前已有待执行恢复计划 {}，请先重启服务后再安排新的恢复。",
            pending.filename
        )
    })
}
