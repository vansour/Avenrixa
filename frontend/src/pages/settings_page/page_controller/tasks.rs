use crate::action_feedback::{set_action_error, spawn_tracked_action};
use crate::types::api::UpdateAdminSettingsConfigRequest;
use dioxus::prelude::WritableExt;
use gloo_timers::future::TimeoutFuture;

use super::{
    SETTINGS_LOAD_RETRY_DELAYS_MS, SettingsPageController, handle_settings_auth_error,
    settings_auth_expired_message,
};

pub(super) fn spawn_reload_admin_settings(controller: SettingsPageController) {
    if !controller.is_admin {
        let mut is_loading = controller.is_loading;
        let mut error_message = controller.error_message;
        is_loading.set(false);
        error_message.set(String::new());
        return;
    }

    let settings_service = controller.settings_service.clone();
    let auth_store = controller.auth_store.clone();
    let toast_store = controller.toast_store.clone();
    let mut loaded_config = controller.loaded_config;
    let mut form = controller.form;
    let mut error_message = controller.error_message;

    spawn_tracked_action(
        controller.is_loading,
        controller.error_message,
        async move {
            let mut last_error = None;

            for delay_ms in SETTINGS_LOAD_RETRY_DELAYS_MS {
                if delay_ms > 0 {
                    TimeoutFuture::new(delay_ms).await;
                }

                match settings_service.get_admin_settings_config().await {
                    Ok(config) => {
                        loaded_config.set(Some(config.clone()));
                        form.apply_loaded_config(config);
                        return;
                    }
                    Err(err) if err.should_redirect_login() => {
                        last_error = Some(err);
                        break;
                    }
                    Err(err) => last_error = Some(err),
                }
            }

            if let Some(err) = last_error {
                if handle_settings_auth_error(&auth_store, &toast_store, &err) {
                    error_message.set(settings_auth_expired_message());
                } else {
                    set_action_error(
                        error_message,
                        &toast_store,
                        format!("加载设置失败: {}", err),
                    );
                }
            }
        },
    );
}

pub(super) fn spawn_save_settings(
    controller: SettingsPageController,
    req: UpdateAdminSettingsConfigRequest,
) {
    let settings_service = controller.settings_service.clone();
    let auth_store = controller.auth_store.clone();
    let toast_store = controller.toast_store.clone();
    let mut loaded_config = controller.loaded_config;
    let mut form = controller.form;
    let on_site_name_updated = controller.on_site_name_updated;
    let mut error_message = controller.error_message;

    spawn_tracked_action(controller.is_saving, controller.error_message, async move {
        match settings_service.update_admin_settings_config(req).await {
            Ok(config) => {
                loaded_config.set(Some(config.clone()));
                form.apply_loaded_config(config.clone());
                on_site_name_updated.call(config.site_name.clone());
                toast_store.show_success("设置已保存".to_string());
            }
            Err(err) => {
                if handle_settings_auth_error(&auth_store, &toast_store, &err) {
                    error_message.set(settings_auth_expired_message());
                } else {
                    set_action_error(
                        error_message,
                        &toast_store,
                        format!("保存设置失败: {}", err),
                    );
                }
            }
        }
    });
}
