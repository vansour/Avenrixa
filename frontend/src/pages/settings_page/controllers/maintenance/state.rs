use crate::services::AdminService;
use crate::types::api::{BackupFileSummary, BackupResponse};
use dioxus::prelude::*;

use super::super::{MaintenanceAction, PendingMaintenanceAction, maintenance_confirmation_plan};
use super::helpers::{build_backup_downloads, combined_error_message, is_maintenance_busy};

#[derive(Clone, Copy)]
pub(super) struct MaintenanceState {
    pub(super) error_message: Signal<String>,
    pub(super) backup_list_error_message: Signal<String>,
    pub(super) success_message: Signal<String>,
    pub(super) last_backup: Signal<Option<BackupResponse>>,
    pub(super) backup_files: Signal<Vec<BackupFileSummary>>,
    pub(super) last_expired_cleanup_count: Signal<Option<i64>>,
    pub(super) is_cleaning_expired: Signal<bool>,
    pub(super) is_backing_up: Signal<bool>,
    pub(super) deleting_backup_filename: Signal<Option<String>>,
    pub(super) is_loading_backups: Signal<bool>,
    pub(super) reload_backups_tick: Signal<u64>,
    pub(super) pending_action: Signal<Option<PendingMaintenanceAction>>,
}

pub(super) fn use_maintenance_state() -> MaintenanceState {
    MaintenanceState {
        error_message: use_signal(String::new),
        backup_list_error_message: use_signal(String::new),
        success_message: use_signal(String::new),
        last_backup: use_signal(|| None::<BackupResponse>),
        backup_files: use_signal(Vec::<BackupFileSummary>::new),
        last_expired_cleanup_count: use_signal(|| None::<i64>),
        is_cleaning_expired: use_signal(|| false),
        is_backing_up: use_signal(|| false),
        deleting_backup_filename: use_signal(|| None::<String>),
        is_loading_backups: use_signal(|| false),
        reload_backups_tick: use_signal(|| 0_u64),
        pending_action: use_signal(|| None::<PendingMaintenanceAction>),
    }
}

impl MaintenanceState {
    pub(super) fn is_busy(self) -> bool {
        is_maintenance_busy(
            self.is_cleaning_expired,
            self.is_backing_up,
            self.deleting_backup_filename,
        )
    }

    pub(super) fn combined_error_message(self) -> String {
        combined_error_message(self.error_message, self.backup_list_error_message)
    }

    pub(super) fn backup_downloads(
        self,
        admin_service: &AdminService,
    ) -> Vec<(BackupFileSummary, String)> {
        build_backup_downloads(admin_service, self.backup_files)
    }

    pub(super) fn success_message(self) -> String {
        let success_message = self.success_message;
        success_message()
    }

    pub(super) fn last_backup(self) -> Option<BackupResponse> {
        let last_backup = self.last_backup;
        last_backup()
    }

    pub(super) fn last_expired_cleanup_count(self) -> Option<i64> {
        let last_expired_cleanup_count = self.last_expired_cleanup_count;
        last_expired_cleanup_count()
    }

    pub(super) fn is_cleaning_expired(self) -> bool {
        let is_cleaning_expired = self.is_cleaning_expired;
        is_cleaning_expired()
    }

    pub(super) fn is_backing_up(self) -> bool {
        let is_backing_up = self.is_backing_up;
        is_backing_up()
    }

    pub(super) fn deleting_backup_filename(self) -> Option<String> {
        let deleting_backup_filename = self.deleting_backup_filename;
        deleting_backup_filename()
    }

    pub(super) fn is_loading_backups(self) -> bool {
        let is_loading_backups = self.is_loading_backups;
        is_loading_backups()
    }

    pub(super) fn request_cleanup_confirmation(self) {
        let mut pending_action = self.pending_action;
        pending_action.set(Some(PendingMaintenanceAction {
            action: MaintenanceAction::CleanupExpired,
            plan: maintenance_confirmation_plan(MaintenanceAction::CleanupExpired),
        }));
    }

    pub(super) fn request_delete_backup_confirmation(self, filename: String) {
        let action = MaintenanceAction::DeleteBackup(filename);
        let mut pending_action = self.pending_action;
        pending_action.set(Some(PendingMaintenanceAction {
            action: action.clone(),
            plan: maintenance_confirmation_plan(action),
        }));
    }

    pub(super) fn refresh_backups(self) {
        let mut reload_backups_tick = self.reload_backups_tick;
        reload_backups_tick.set(reload_backups_tick().wrapping_add(1));
    }

    pub(super) fn clear_pending_action(self) {
        let mut pending_action = self.pending_action;
        pending_action.set(None);
    }

    pub(super) fn pending_action(self) -> Option<PendingMaintenanceAction> {
        let pending_action = self.pending_action;
        pending_action()
    }
}
