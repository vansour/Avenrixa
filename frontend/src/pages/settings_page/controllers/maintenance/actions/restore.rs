use crate::services::AdminService;
use crate::store::{AuthStore, ToastStore};
use crate::types::api::PendingBackupRestore;
use dioxus::prelude::*;

use super::super::super::{
    MaintenanceAction, PendingMaintenanceAction, restore_confirmation_plan,
    restore_precheck_error_message, set_settings_action_error,
};

#[allow(clippy::too_many_arguments)]
pub(crate) async fn schedule_restore(
    admin_service: AdminService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    filename: String,
    mut processing_restore_filename: Signal<Option<String>>,
    mut error_message: Signal<String>,
    mut success_message: Signal<String>,
    mut reload_restore_status_tick: Signal<u64>,
) {
    processing_restore_filename.set(Some(filename.clone()));
    error_message.set(String::new());
    success_message.set(String::new());

    match admin_service.schedule_backup_restore(&filename).await {
        Ok(response) => {
            reload_restore_status_tick.set(reload_restore_status_tick().wrapping_add(1));
            set_restore_scheduled_success(success_message, toast_store, response.pending);
        }
        Err(err) => {
            set_settings_action_error(
                &auth_store,
                &toast_store,
                error_message,
                &err,
                "写入恢复计划失败",
            );
        }
    }

    processing_restore_filename.set(None);
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn precheck_restore(
    admin_service: AdminService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    filename: String,
    mut processing_restore_filename: Signal<Option<String>>,
    mut error_message: Signal<String>,
    mut success_message: Signal<String>,
    mut pending_action: Signal<Option<PendingMaintenanceAction>>,
) {
    processing_restore_filename.set(Some(filename.clone()));
    error_message.set(String::new());
    success_message.set(String::new());

    match admin_service.precheck_backup_restore(&filename).await {
        Ok(precheck) => {
            if precheck.eligible {
                pending_action.set(Some(PendingMaintenanceAction {
                    action: MaintenanceAction::RestoreBackup(filename),
                    plan: restore_confirmation_plan(&precheck),
                }));
            } else {
                let message = restore_precheck_error_message(&precheck);
                error_message.set(message.clone());
                toast_store.show_error(message);
            }
        }
        Err(err) => {
            set_settings_action_error(
                &auth_store,
                &toast_store,
                error_message,
                &err,
                "恢复预检失败",
            );
        }
    }

    processing_restore_filename.set(None);
}

fn set_restore_scheduled_success(
    mut success_message: Signal<String>,
    toast_store: ToastStore,
    pending: PendingBackupRestore,
) {
    let message = format!("恢复计划已写入，请立即重启服务：{}", pending.filename);
    success_message.set(message.clone());
    toast_store.show_success(message);
}
