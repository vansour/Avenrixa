use crate::app_context::{use_auth_service, use_auth_store, use_toast_store};
use crate::types::api::UserRole;
use dioxus::prelude::*;

use super::super::handle_settings_auth_error;
use super::super::view::AccountSettingsSection;

#[component]
pub fn AccountSectionController() -> Element {
    let auth_service = use_auth_service();
    let auth_store = use_auth_store();
    let toast_store = use_toast_store();

    let mut is_logging_out = use_signal(|| false);

    let current_user = auth_store.user();
    let email = current_user
        .as_ref()
        .map(|user| user.email.clone())
        .unwrap_or_else(|| "当前用户".to_string());
    let role = current_user
        .as_ref()
        .map(|user| user.role)
        .unwrap_or(UserRole::User);
    let created_at = current_user
        .as_ref()
        .map(|user| user.created_at.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "-".to_string());

    let auth_service_for_logout = auth_service.clone();
    let auth_store_for_logout = auth_store.clone();
    let toast_store_for_logout = toast_store.clone();
    let handle_logout = move |_| {
        if is_logging_out() {
            return;
        }

        let auth_service = auth_service_for_logout.clone();
        let auth_store = auth_store_for_logout.clone();
        let toast_store = toast_store_for_logout.clone();
        spawn(async move {
            is_logging_out.set(true);

            match auth_service.logout().await {
                Ok(_) => {
                    toast_store.show_success("已退出登录".to_string());
                }
                Err(err) => {
                    if handle_settings_auth_error(&auth_store, &toast_store, &err) {
                        is_logging_out.set(false);
                        return;
                    }
                    let message = format!("退出登录失败: {}", err);
                    toast_store.show_error(message.clone());
                    eprintln!("{message}");
                }
            }

            is_logging_out.set(false);
        });
    };

    rsx! {
        AccountSettingsSection {
            email,
            role,
            created_at,
            is_logging_out: is_logging_out(),
            on_logout: handle_logout,
        }
    }
}
