use crate::app_context::{use_auth_store, use_settings_service, use_toast_store};
use crate::auth_session::{auth_session_expired_message, handle_auth_session_error};
use crate::services::SettingsService;
use crate::store::{AuthStore, ToastStore};
use crate::types::api::AdminSettingsConfig;
use crate::types::errors::AppError;
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use super::state::SettingsFormState;

const SETTINGS_LOAD_RETRY_DELAYS_MS: [u32; 3] = [0, 500, 1500];

pub(crate) fn settings_auth_expired_message() -> String {
    auth_session_expired_message()
}

pub(crate) fn handle_settings_auth_error(
    auth_store: &AuthStore,
    toast_store: &ToastStore,
    err: &AppError,
) -> bool {
    handle_auth_session_error(auth_store, toast_store, err)
}

#[derive(Clone)]
pub(super) struct SettingsPageController {
    pub form: SettingsFormState,
    pub is_loading: Signal<bool>,
    pub is_saving: Signal<bool>,
    pub error_message: Signal<String>,
    pub loaded_config: Signal<Option<AdminSettingsConfig>>,
    reload_tick: Signal<u64>,
    last_loaded_tick: Signal<Option<u64>>,
    settings_service: SettingsService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    on_site_name_updated: EventHandler<String>,
    is_admin: bool,
}

pub(super) fn use_settings_page_controller(
    is_admin: bool,
    on_site_name_updated: EventHandler<String>,
) -> SettingsPageController {
    let settings_service = use_settings_service();
    let auth_store = use_auth_store();
    let toast_store = use_toast_store();

    let is_loading = use_signal(|| true);
    let is_saving = use_signal(|| false);
    let error_message = use_signal(String::new);
    let loaded_config = use_signal(|| None::<AdminSettingsConfig>);
    let reload_tick = use_signal(|| 0_u64);
    let last_loaded_tick = use_signal(|| None::<u64>);

    let site_name = use_signal(String::new);
    let storage_backend = use_signal(|| crate::types::api::StorageBackendKind::Unknown);
    let local_storage_path = use_signal(String::new);
    let mail_enabled = use_signal(|| false);
    let mail_smtp_host = use_signal(String::new);
    let mail_smtp_port = use_signal(String::new);
    let mail_smtp_user = use_signal(String::new);
    let mail_smtp_password = use_signal(String::new);
    let mail_smtp_password_set = use_signal(|| false);
    let mail_from_email = use_signal(String::new);
    let mail_from_name = use_signal(String::new);
    let mail_link_base_url = use_signal(String::new);

    let form = SettingsFormState {
        site_name,
        storage_backend,
        local_storage_path,
        mail_enabled,
        mail_smtp_host,
        mail_smtp_port,
        mail_smtp_user,
        mail_smtp_password,
        mail_smtp_password_set,
        mail_from_email,
        mail_from_name,
        mail_link_base_url,
    };

    let controller = SettingsPageController {
        form,
        is_loading,
        is_saving,
        error_message,
        loaded_config,
        reload_tick,
        last_loaded_tick,
        settings_service,
        auth_store,
        toast_store,
        on_site_name_updated,
        is_admin,
    };

    use_effect({
        let controller = controller.clone();
        move || {
            let current_tick = (controller.reload_tick)();
            if (controller.last_loaded_tick)() == Some(current_tick) {
                return;
            }
            let mut last_loaded_tick = controller.last_loaded_tick;
            last_loaded_tick.set(Some(current_tick));
            let controller_for_reload = controller.clone();
            spawn(async move {
                controller_for_reload.reload_admin_settings().await;
            });
        }
    });

    controller
}

impl SettingsPageController {
    pub fn is_loading(&self) -> bool {
        (self.is_loading)()
    }

    pub fn is_saving(&self) -> bool {
        (self.is_saving)()
    }

    pub fn error_message(&self) -> String {
        (self.error_message)()
    }

    pub fn loaded_config(&self) -> Option<AdminSettingsConfig> {
        (self.loaded_config)()
    }

    pub fn refresh(&self) {
        if self.is_loading() {
            return;
        }

        let mut reload_tick = self.reload_tick;
        reload_tick.set((self.reload_tick)().wrapping_add(1));
    }

    pub fn save(&self) {
        if self.is_saving() {
            return;
        }

        if let Err(message) = self.form.validate() {
            let mut error_message = self.error_message;
            error_message.set(message.clone());
            self.toast_store.show_error(message);
            return;
        }

        let req = self.form.build_update_request(
            self.loaded_config()
                .as_ref()
                .map(|config| config.settings_version.clone()),
        );
        let controller = self.clone();
        spawn(async move {
            let mut is_saving = controller.is_saving;
            let mut error_message = controller.error_message;
            let mut loaded_config = controller.loaded_config;

            is_saving.set(true);
            error_message.set(String::new());

            match controller
                .settings_service
                .update_admin_settings_config(req)
                .await
            {
                Ok(config) => {
                    loaded_config.set(Some(config.clone()));
                    let mut form = controller.form;
                    form.apply_loaded_config(config.clone());
                    controller
                        .on_site_name_updated
                        .call(config.site_name.clone());
                    controller
                        .toast_store
                        .show_success("设置已保存".to_string());
                }
                Err(err) => {
                    if handle_settings_auth_error(
                        &controller.auth_store,
                        &controller.toast_store,
                        &err,
                    ) {
                        error_message.set(settings_auth_expired_message());
                    } else {
                        let message = format!("保存设置失败: {}", err);
                        error_message.set(message.clone());
                        controller.toast_store.show_error(message);
                    }
                }
            }

            is_saving.set(false);
        });
    }

    async fn reload_admin_settings(&self) {
        if !self.is_admin {
            let mut is_loading = self.is_loading;
            let mut error_message = self.error_message;

            is_loading.set(false);
            error_message.set(String::new());
            return;
        }

        let mut is_loading = self.is_loading;
        let mut error_message = self.error_message;
        let mut loaded_config = self.loaded_config;

        is_loading.set(true);
        error_message.set(String::new());
        let mut last_error = None;

        for delay_ms in SETTINGS_LOAD_RETRY_DELAYS_MS {
            if delay_ms > 0 {
                TimeoutFuture::new(delay_ms).await;
            }

            match self.settings_service.get_admin_settings_config().await {
                Ok(config) => {
                    loaded_config.set(Some(config.clone()));
                    let mut form = self.form;
                    form.apply_loaded_config(config);
                    is_loading.set(false);
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
            if handle_settings_auth_error(&self.auth_store, &self.toast_store, &err) {
                error_message.set(settings_auth_expired_message());
            } else {
                let message = format!("加载设置失败: {}", err);
                error_message.set(message.clone());
                self.toast_store.show_error(message);
            }
        }

        is_loading.set(false);
    }
}
