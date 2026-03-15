use crate::services::AdminService;
use crate::store::{AuthStore, ToastStore};
use crate::types::api::Setting;
use dioxus::prelude::*;
use std::collections::HashMap;

use super::super::set_settings_load_error;

pub(super) fn use_raw_settings_loader(
    admin_service: AdminService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    settings: Signal<Vec<Setting>>,
    setting_drafts: Signal<HashMap<String, String>>,
    error_message: Signal<String>,
    is_loading: Signal<bool>,
    reload_tick: Signal<u64>,
) {
    let _ = use_resource(move || {
        let _ = reload_tick();
        let admin_service = admin_service.clone();
        let auth_store = auth_store.clone();
        let toast_store = toast_store.clone();
        async move {
            load_raw_settings(
                admin_service,
                auth_store,
                toast_store,
                settings,
                setting_drafts,
                error_message,
                is_loading,
            )
            .await;
        }
    });
}

async fn load_raw_settings(
    admin_service: AdminService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    mut settings: Signal<Vec<Setting>>,
    mut setting_drafts: Signal<HashMap<String, String>>,
    mut error_message: Signal<String>,
    mut is_loading: Signal<bool>,
) {
    is_loading.set(true);
    error_message.set(String::new());

    match admin_service.get_raw_settings().await {
        Ok(result) => {
            let draft_map = result
                .iter()
                .map(|setting| (setting.key.clone(), setting.value.clone()))
                .collect();
            settings.set(result);
            setting_drafts.set(draft_map);
        }
        Err(err) => {
            set_settings_load_error(
                &auth_store,
                &toast_store,
                error_message,
                &err,
                "加载原始设置失败",
            );
        }
    }

    is_loading.set(false);
}
