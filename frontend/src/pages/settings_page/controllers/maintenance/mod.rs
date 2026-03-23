mod actions;
mod handlers;
mod helpers;
mod loaders;
mod modal;
mod state;

use crate::app_context::{use_admin_service, use_auth_store, use_toast_store};
use dioxus::prelude::*;

use self::handlers::{
    confirm_pending_action, handle_backup_database, handle_cleanup_expired, handle_delete_backup,
    handle_refresh_backups,
};
use self::loaders::use_backups_loader;
use self::modal::PendingMaintenanceActionModal;
use self::state::use_maintenance_state;
use super::super::view::MaintenanceSettingsSection;

#[component]
pub fn MaintenanceSectionController() -> Element {
    let admin_service = use_admin_service();
    let auth_store = use_auth_store();
    let toast_store = use_toast_store();

    let state = use_maintenance_state();

    use_backups_loader(
        admin_service.clone(),
        auth_store.clone(),
        toast_store.clone(),
        state.backup_files,
        state.last_backup,
        state.backup_list_error_message,
        state.is_loading_backups,
        state.reload_backups_tick,
    );

    let on_cleanup_expired = handle_cleanup_expired(state);
    let on_backup = handle_backup_database(
        admin_service.clone(),
        auth_store.clone(),
        toast_store.clone(),
        state,
    );
    let on_refresh_backups = handle_refresh_backups(state);
    let on_delete_backup = handle_delete_backup(state);
    let backup_downloads = state.backup_downloads(&admin_service);

    rsx! {
        MaintenanceSettingsSection {
            error_message: state.combined_error_message(),
            success_message: state.success_message(),
            last_backup: state.last_backup(),
            backup_files: backup_downloads,
            last_expired_cleanup_count: state.last_expired_cleanup_count(),
            is_cleaning_expired: state.is_cleaning_expired(),
            is_backing_up: state.is_backing_up(),
            deleting_backup_filename: state.deleting_backup_filename(),
            is_loading_backups: state.is_loading_backups(),
            on_cleanup_expired,
            on_backup,
            on_refresh_backups,
            on_delete_backup,
        }

        if let Some(pending) = state.pending_action() {
            PendingMaintenanceActionModal {
                pending,
                is_submitting: state.is_busy(),
                on_close: move || state.clear_pending_action(),
                on_confirm_action: move |action| {
                    confirm_pending_action(
                        admin_service.clone(),
                        auth_store.clone(),
                        toast_store.clone(),
                        state,
                        action,
                    );
                },
            }
        }
    }
}
