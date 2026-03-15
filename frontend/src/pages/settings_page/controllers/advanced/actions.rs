use crate::services::AdminService;
use crate::store::{AuthStore, ToastStore};
use crate::types::api::Setting;
use dioxus::prelude::*;
use std::collections::HashMap;

use super::super::{
    PendingSettingChange, advanced_setting_confirmation_plan, set_settings_action_error,
    setting_is_high_risk,
};

pub(super) enum SettingSavePlan {
    Error(String),
    Info(String),
    RequiresConfirmation(PendingSettingChange),
    SaveNow(String),
}

pub(super) fn plan_setting_save(
    settings: &[Setting],
    setting_drafts: &HashMap<String, String>,
    key: &str,
) -> SettingSavePlan {
    let Some(current_setting) = settings.iter().find(|setting| setting.key == key) else {
        return SettingSavePlan::Error("未找到要更新的设置项".to_string());
    };

    if !current_setting.editable {
        return SettingSavePlan::Error(format!("设置项 {} 受保护，不能通过高级设置直接修改", key));
    }

    let next_value = setting_drafts
        .get(key)
        .cloned()
        .unwrap_or_else(|| current_setting.value.clone());

    if next_value == current_setting.value {
        return SettingSavePlan::Info(format!("键值 {} 未发生变化", key));
    }

    if current_setting.requires_confirmation {
        let plan = advanced_setting_confirmation_plan(key, &next_value);
        return SettingSavePlan::RequiresConfirmation(PendingSettingChange {
            key: key.to_string(),
            next_value,
            plan,
        });
    }

    SettingSavePlan::SaveNow(next_value)
}

pub(super) async fn save_setting(
    admin_service: AdminService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    key: String,
    next_value: String,
    on_site_name_updated: EventHandler<String>,
    mut saving_key: Signal<Option<String>>,
    mut error_message: Signal<String>,
    mut success_message: Signal<String>,
    mut reload_tick: Signal<u64>,
) {
    saving_key.set(Some(key.clone()));
    error_message.set(String::new());
    success_message.set(String::new());

    match admin_service.update_setting(&key, next_value.clone()).await {
        Ok(_) => {
            let message = format!("已更新原始设置 {}", key);
            success_message.set(message.clone());
            toast_store.show_success(message);
            if setting_is_high_risk(&key) {
                toast_store.show_info("存储配置已立即应用，建议立刻检查健康状态".to_string());
            }
            if key == "site_name" {
                on_site_name_updated.call(next_value.trim().to_string());
            }
            reload_tick.set(reload_tick().wrapping_add(1));
        }
        Err(err) => {
            set_settings_action_error(
                &auth_store,
                &toast_store,
                error_message,
                &err,
                "更新原始设置失败",
            );
        }
    }

    saving_key.set(None);
}
