use crate::services::SettingsService;
use crate::store::{AuthStore, ToastStore};
use crate::types::api::StorageDirectoryEntry;
use dioxus::prelude::*;

use super::super::super::super::{handle_settings_auth_error, settings_auth_expired_message};

pub(super) const DEFAULT_SETTINGS_STORAGE_BROWSER_PATH: &str = "/";

pub(super) fn load_settings_storage_directories(
    settings_service: SettingsService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    browser_signals: StorageBrowserSignals,
    requested_path: String,
) {
    let requested_path = if requested_path.trim().is_empty() {
        DEFAULT_SETTINGS_STORAGE_BROWSER_PATH.to_string()
    } else {
        requested_path
    };

    spawn(async move {
        let StorageBrowserSignals {
            mut loading,
            error: mut error_signal,
            mut current_path,
            mut parent_path,
            mut directories,
        } = browser_signals;

        loading.set(true);
        error_signal.set(String::new());

        match settings_service
            .browse_storage_directories(Some(requested_path.as_str()))
            .await
        {
            Ok(response) => {
                current_path.set(response.current_path);
                parent_path.set(response.parent_path);
                directories.set(response.directories);
            }
            Err(err) => {
                if handle_settings_auth_error(&auth_store, &toast_store, &err) {
                    error_signal.set(settings_auth_expired_message());
                } else {
                    error_signal.set(format!("读取目录失败：{}", err));
                }
            }
        }

        loading.set(false);
    });
}

#[derive(Clone, Copy)]
pub(super) struct StorageBrowserSignals {
    pub(super) loading: Signal<bool>,
    pub(super) error: Signal<String>,
    pub(super) current_path: Signal<String>,
    pub(super) parent_path: Signal<Option<String>>,
    pub(super) directories: Signal<Vec<StorageDirectoryEntry>>,
}
