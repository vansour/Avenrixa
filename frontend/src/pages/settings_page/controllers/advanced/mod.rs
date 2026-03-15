mod actions;
mod loaders;

use crate::app_context::{use_admin_service, use_auth_store, use_toast_store};
use crate::components::ConfirmationModal;
use crate::types::api::Setting;
use dioxus::prelude::*;
use std::collections::HashMap;

use self::actions::{SettingSavePlan, plan_setting_save, save_setting};
use self::loaders::use_raw_settings_loader;
use super::super::view::AdvancedSettingsSection;
use super::PendingSettingChange;

#[component]
pub fn AdvancedSectionController(
    #[props(default)] on_site_name_updated: EventHandler<String>,
) -> Element {
    let admin_service = use_admin_service();
    let auth_store = use_auth_store();
    let toast_store = use_toast_store();

    let settings = use_signal(Vec::<Setting>::new);
    let setting_drafts = use_signal(HashMap::<String, String>::new);
    let mut error_message = use_signal(String::new);
    let mut success_message = use_signal(String::new);
    let is_loading = use_signal(|| false);
    let mut reload_tick = use_signal(|| 0_u64);
    let saving_key = use_signal(|| None::<String>);
    let mut pending_setting_change = use_signal(|| None::<PendingSettingChange>);

    use_raw_settings_loader(
        admin_service.clone(),
        auth_store.clone(),
        toast_store.clone(),
        settings,
        setting_drafts,
        error_message,
        is_loading,
        reload_tick,
    );

    let handle_refresh = move |_| {
        if is_loading() || saving_key().is_some() {
            return;
        }
        reload_tick.set(reload_tick().wrapping_add(1));
    };

    let admin_service_for_raw_setting = admin_service.clone();
    let auth_store_for_raw_setting = auth_store.clone();
    let toast_store_for_raw_setting = toast_store.clone();
    let on_site_name_updated_for_raw_setting = on_site_name_updated;
    let admin_service_for_confirm_setting = admin_service.clone();
    let auth_store_for_confirm_setting = auth_store.clone();
    let toast_store_for_confirm_setting = toast_store.clone();
    let on_site_name_updated_for_confirm_setting = on_site_name_updated;
    let handle_save_setting = move |key: String| {
        if saving_key().is_some() {
            return;
        }

        let current_settings = settings();
        let current_drafts = setting_drafts();
        match plan_setting_save(&current_settings, &current_drafts, &key) {
            SettingSavePlan::Error(message) => {
                error_message.set(message.clone());
                toast_store_for_raw_setting.show_error(message);
            }
            SettingSavePlan::Info(message) => {
                success_message.set(message.clone());
                toast_store_for_raw_setting.show_info(message);
            }
            SettingSavePlan::RequiresConfirmation(pending) => {
                pending_setting_change.set(Some(pending));
            }
            SettingSavePlan::SaveNow(next_value) => {
                let admin_service = admin_service_for_raw_setting.clone();
                let auth_store = auth_store_for_raw_setting.clone();
                let toast_store = toast_store_for_raw_setting.clone();
                let on_site_name_updated = on_site_name_updated_for_raw_setting;
                spawn(async move {
                    save_setting(
                        admin_service,
                        auth_store,
                        toast_store,
                        key,
                        next_value,
                        on_site_name_updated,
                        saving_key,
                        error_message,
                        success_message,
                        reload_tick,
                    )
                    .await;
                });
            }
        }
    };

    rsx! {
        AdvancedSettingsSection {
            settings: settings(),
            setting_drafts,
            error_message: error_message(),
            success_message: success_message(),
            is_loading: is_loading(),
            saving_key: saving_key(),
            on_refresh: handle_refresh,
            on_save_setting: handle_save_setting,
        }

        if let Some(pending) = pending_setting_change() {
            ConfirmationModal {
                title: pending.plan.title.clone(),
                summary: pending.plan.summary.clone(),
                consequences: pending.plan.consequences.clone(),
                confirm_label: pending.plan.confirm_label.clone(),
                cancel_label: pending.plan.cancel_label.clone(),
                tone: pending.plan.tone,
                confirm_phrase: pending.plan.confirm_phrase.clone(),
                confirm_hint: pending.plan.confirm_hint.clone(),
                is_submitting: saving_key().is_some(),
                on_close: move |_| pending_setting_change.set(None),
                on_confirm: move |_| {
                    let key = pending.key.clone();
                    let next_value = pending.next_value.clone();
                    pending_setting_change.set(None);

                    let admin_service = admin_service_for_confirm_setting.clone();
                    let auth_store = auth_store_for_confirm_setting.clone();
                    let toast_store = toast_store_for_confirm_setting.clone();
                    let on_site_name_updated = on_site_name_updated_for_confirm_setting;
                    spawn(async move {
                        save_setting(
                            admin_service,
                            auth_store,
                            toast_store,
                            key,
                            next_value,
                            on_site_name_updated,
                            saving_key,
                            error_message,
                            success_message,
                            reload_tick,
                        )
                        .await;
                    });
                },
            }
        }
    }
}
