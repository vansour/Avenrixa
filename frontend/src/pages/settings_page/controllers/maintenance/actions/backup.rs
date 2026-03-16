use crate::services::AdminService;
use crate::store::{AuthStore, ToastStore};
use crate::types::api::BackupResponse;
use dioxus::prelude::*;

use super::super::super::set_settings_action_error;

pub(crate) async fn trigger_cleanup_expired(
    admin_service: AdminService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    mut is_cleaning_expired: Signal<bool>,
    mut error_message: Signal<String>,
    mut success_message: Signal<String>,
    mut last_expired_cleanup_count: Signal<Option<i64>>,
) {
    is_cleaning_expired.set(true);
    error_message.set(String::new());
    success_message.set(String::new());

    match admin_service.cleanup_expired_images().await {
        Ok(affected) => {
            last_expired_cleanup_count.set(Some(affected));
            let message = if affected <= 0 {
                "当前没有需要永久删除的过期图片".to_string()
            } else {
                format!("已永久删除 {} 张过期图片", affected)
            };
            success_message.set(message.clone());
            toast_store.show_success(message);
        }
        Err(err) => {
            set_settings_action_error(
                &auth_store,
                &toast_store,
                error_message,
                &err,
                "永久删除过期图片失败",
            );
        }
    }

    is_cleaning_expired.set(false);
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn backup_database(
    admin_service: AdminService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    mut is_backing_up: Signal<bool>,
    mut error_message: Signal<String>,
    mut success_message: Signal<String>,
    mut last_backup: Signal<Option<BackupResponse>>,
    mut reload_backups_tick: Signal<u64>,
) {
    is_backing_up.set(true);
    error_message.set(String::new());
    success_message.set(String::new());

    match admin_service.backup_database().await {
        Ok(backup) => {
            let filename = backup.filename.clone();
            last_backup.set(Some(backup));
            reload_backups_tick.set(reload_backups_tick().wrapping_add(1));
            let message = format!("已生成数据库备份: {}", filename);
            success_message.set(message.clone());
            toast_store.show_success(message);
        }
        Err(err) => {
            set_settings_action_error(
                &auth_store,
                &toast_store,
                error_message,
                &err,
                "数据库备份失败",
            );
        }
    }

    is_backing_up.set(false);
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn delete_backup(
    admin_service: AdminService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    filename: String,
    mut deleting_backup_filename: Signal<Option<String>>,
    mut error_message: Signal<String>,
    mut success_message: Signal<String>,
    mut reload_backups_tick: Signal<u64>,
) {
    deleting_backup_filename.set(Some(filename.clone()));
    error_message.set(String::new());
    success_message.set(String::new());

    match admin_service.delete_backup(&filename).await {
        Ok(_) => {
            let message = format!("已删除备份文件: {}", filename);
            success_message.set(message.clone());
            toast_store.show_success(message);
            reload_backups_tick.set(reload_backups_tick().wrapping_add(1));
        }
        Err(err) => {
            set_settings_action_error(
                &auth_store,
                &toast_store,
                error_message,
                &err,
                "删除备份文件失败",
            );
        }
    }

    deleting_backup_filename.set(None);
}
