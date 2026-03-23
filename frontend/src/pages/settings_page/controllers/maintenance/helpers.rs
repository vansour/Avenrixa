use crate::services::AdminService;
use crate::types::api::BackupFileSummary;
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
) -> String {
    merge_messages(&error_message(), &backup_list_error_message())
}

pub(super) fn is_maintenance_busy(
    is_cleaning_expired: Signal<bool>,
    is_backing_up: Signal<bool>,
    deleting_backup_filename: Signal<Option<String>>,
) -> bool {
    is_cleaning_expired() || is_backing_up() || deleting_backup_filename().is_some()
}
