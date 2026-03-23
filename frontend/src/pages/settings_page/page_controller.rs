mod form;
mod tasks;

use crate::action_feedback::set_action_error;
use crate::app_context::{use_auth_store, use_settings_service, use_toast_store};
use crate::auth_session::{auth_session_expired_message, handle_auth_session_error};
use crate::services::SettingsService;
use crate::store::{AuthStore, ToastStore};
use crate::types::api::AdminSettingsConfig;
use crate::types::errors::AppError;
use dioxus::prelude::*;

use super::state::SettingsFormState;
use form::build_settings_form;
use tasks::{spawn_reload_admin_settings, spawn_save_settings};

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

    let controller = SettingsPageController {
        form: build_settings_form(),
        is_loading: use_signal(|| true),
        is_saving: use_signal(|| false),
        error_message: use_signal(String::new),
        loaded_config: use_signal(|| None::<AdminSettingsConfig>),
        reload_tick: use_signal(|| 0_u64),
        last_loaded_tick: use_signal(|| None::<u64>),
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
            spawn_reload_admin_settings(controller.clone());
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
            set_action_error(self.error_message, &self.toast_store, message);
            return;
        }

        let req = self.form.build_update_request(
            self.loaded_config()
                .as_ref()
                .map(|config| config.settings_version.clone()),
        );

        spawn_save_settings(self.clone(), req);
    }
}
