use crate::services::AdminService;
use crate::store::{AuthStore, ToastStore};
use dioxus::prelude::*;

use super::super::MaintenanceAction;
use super::actions::{backup_database, delete_backup, trigger_cleanup_expired};
use super::state::MaintenanceState;

pub(super) fn handle_cleanup_expired(state: MaintenanceState) -> impl FnMut(MouseEvent) + 'static {
    move |_| {
        if state.is_busy() {
            return;
        }

        state.request_cleanup_confirmation();
    }
}

pub(super) fn handle_backup_database(
    admin_service: AdminService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    state: MaintenanceState,
) -> impl FnMut(MouseEvent) + 'static {
    move |_| {
        if state.is_busy() {
            return;
        }

        let admin_service = admin_service.clone();
        let auth_store = auth_store.clone();
        let toast_store = toast_store.clone();
        spawn(async move {
            backup_database(
                admin_service,
                auth_store,
                toast_store,
                state.is_backing_up,
                state.error_message,
                state.success_message,
                state.last_backup,
                state.reload_backups_tick,
            )
            .await;
        });
    }
}

pub(super) fn handle_delete_backup(state: MaintenanceState) -> impl FnMut(String) + 'static {
    move |filename| {
        if state.is_busy() {
            return;
        }

        state.request_delete_backup_confirmation(filename);
    }
}

pub(super) fn handle_refresh_backups(state: MaintenanceState) -> impl FnMut(MouseEvent) + 'static {
    move |_| {
        if state.is_busy() || state.is_loading_backups() {
            return;
        }

        state.refresh_backups();
    }
}

pub(super) fn confirm_pending_action(
    admin_service: AdminService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    state: MaintenanceState,
    action: MaintenanceAction,
) {
    state.clear_pending_action();

    match action {
        MaintenanceAction::CleanupExpired => {
            spawn_cleanup_expired(admin_service, auth_store, toast_store, state)
        }
        MaintenanceAction::DeleteBackup(filename) => {
            spawn_delete_backup(admin_service, auth_store, toast_store, state, filename)
        }
    }
}

fn spawn_cleanup_expired(
    admin_service: AdminService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    state: MaintenanceState,
) {
    spawn(async move {
        trigger_cleanup_expired(
            admin_service,
            auth_store,
            toast_store,
            state.is_cleaning_expired,
            state.error_message,
            state.success_message,
            state.last_expired_cleanup_count,
        )
        .await;
    });
}

fn spawn_delete_backup(
    admin_service: AdminService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    state: MaintenanceState,
    filename: String,
) {
    spawn(async move {
        delete_backup(
            admin_service,
            auth_store,
            toast_store,
            filename,
            state.deleting_backup_filename,
            state.error_message,
            state.success_message,
            state.reload_backups_tick,
        )
        .await;
    });
}
