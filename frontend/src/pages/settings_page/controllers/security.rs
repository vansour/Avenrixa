use crate::app_context::{use_auth_service, use_auth_store, use_toast_store};
use crate::types::api::UpdateProfileRequest;
use dioxus::prelude::*;

use super::super::view::SecuritySettingsSection;
use super::set_settings_action_error;

#[component]
pub fn SecuritySectionController() -> Element {
    let auth_service = use_auth_service();
    let auth_store = use_auth_store();
    let toast_store = use_toast_store();

    let mut current_password = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut error_message = use_signal(String::new);
    let mut success_message = use_signal(String::new);
    let mut is_updating_password = use_signal(|| false);
    let current_user_is_admin = auth_store
        .user()
        .as_ref()
        .is_some_and(|user| user.role.is_admin());
    let min_password_length = if current_user_is_admin { 12 } else { 6 };
    let helper_text = if current_user_is_admin {
        "管理员密码至少需要 12 个字符，建议包含大小写字母、数字与符号。".to_string()
    } else {
        "新密码长度需在 6 到 100 个字符之间。".to_string()
    };

    let auth_service_for_password = auth_service.clone();
    let auth_store_for_password = auth_store.clone();
    let toast_store_for_password = toast_store.clone();
    let handle_change_password = move |_| {
        if is_updating_password() {
            return;
        }

        let current = current_password().trim().to_string();
        let next = new_password().trim().to_string();
        let confirm = confirm_password().trim().to_string();

        error_message.set(String::new());
        success_message.set(String::new());

        if current.is_empty() {
            error_message.set("请输入当前密码".to_string());
            return;
        }

        if next.is_empty() {
            error_message.set("请输入新密码".to_string());
            return;
        }

        if !(min_password_length..=100).contains(&next.len()) {
            error_message.set(format!(
                "新密码长度需在 {} 到 100 个字符之间",
                min_password_length
            ));
            return;
        }

        if next != confirm {
            error_message.set("两次输入的新密码不一致".to_string());
            return;
        }

        let auth_service = auth_service_for_password.clone();
        let auth_store = auth_store_for_password.clone();
        let toast_store = toast_store_for_password.clone();
        spawn(async move {
            is_updating_password.set(true);

            let req = UpdateProfileRequest {
                current_password: current,
                new_password: Some(next),
            };

            match auth_service.change_password(req).await {
                Ok(_) => {
                    current_password.set(String::new());
                    new_password.set(String::new());
                    confirm_password.set(String::new());
                    let message = "密码已更新，请重新登录".to_string();
                    success_message.set(message.clone());
                    toast_store.show_success(message);
                }
                Err(err) => {
                    set_settings_action_error(
                        &auth_store,
                        &toast_store,
                        error_message,
                        &err,
                        "修改密码失败",
                    );
                }
            }

            is_updating_password.set(false);
        });
    };

    rsx! {
        SecuritySettingsSection {
            current_password,
            new_password,
            confirm_password,
            error_message: error_message(),
            success_message: success_message(),
            helper_text,
            is_submitting: is_updating_password(),
            on_submit: handle_change_password,
        }
    }
}
