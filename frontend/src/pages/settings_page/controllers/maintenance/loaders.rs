use crate::services::AdminService;
use crate::store::{AuthStore, ToastStore};
use crate::types::api::{BackupFileSummary, BackupResponse};
use dioxus::prelude::*;

use super::super::set_settings_load_error;

#[allow(clippy::let_underscore_future, clippy::too_many_arguments)]
pub(super) fn use_backups_loader(
    admin_service: AdminService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    backup_files: Signal<Vec<BackupFileSummary>>,
    last_backup: Signal<Option<BackupResponse>>,
    backup_list_error_message: Signal<String>,
    is_loading_backups: Signal<bool>,
    reload_backups_tick: Signal<u64>,
) {
    let _ = use_resource(move || {
        let _ = reload_backups_tick();
        let admin_service = admin_service.clone();
        let auth_store = auth_store.clone();
        let toast_store = toast_store.clone();
        async move {
            load_backups(
                admin_service,
                auth_store,
                toast_store,
                is_loading_backups,
                backup_list_error_message,
                backup_files,
                last_backup,
            )
            .await;
        }
    });
}

pub(super) async fn load_backups(
    admin_service: AdminService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    mut is_loading_backups: Signal<bool>,
    mut backup_list_error_message: Signal<String>,
    mut backup_files: Signal<Vec<BackupFileSummary>>,
    mut last_backup: Signal<Option<BackupResponse>>,
) {
    is_loading_backups.set(true);
    backup_list_error_message.set(String::new());

    match admin_service.get_backups().await {
        Ok(result) => {
            let latest_backup = result.first().map(|backup| BackupResponse {
                filename: backup.filename.clone(),
                created_at: backup.created_at,
                semantics: backup.semantics.clone(),
            });
            backup_files.set(result);
            last_backup.set(latest_backup);
        }
        Err(err) => {
            set_settings_load_error(
                &auth_store,
                &toast_store,
                backup_list_error_message,
                &err,
                "加载备份列表失败",
            );
        }
    }

    is_loading_backups.set(false);
}
