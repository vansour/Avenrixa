use crate::app_context::{use_auth_service, use_toast_store};
use crate::types::api::LoginRequest;
use dioxus::prelude::*;

/// 登录页面组件
#[component]
pub fn LoginPage() -> Element {
    let auth_service = use_auth_service();
    let toast_store = use_toast_store();

    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut error_message = use_signal(String::new);

    let handle_login = move |_| {
        let auth_service_clone = auth_service.clone();
        let toast_store = toast_store.clone();
        spawn(async move {
            let username_val = username();
            let password_val = password();

            if username_val.trim().is_empty() || password_val.trim().is_empty() {
                let message = "请输入用户名和密码".to_string();
                error_message.set(message.clone());
                toast_store.show_error(message);
                return;
            }

            is_loading.set(true);
            error_message.set(String::new());

            // 调用登录 API
            match auth_service_clone
                .login(LoginRequest {
                    username: username_val.trim().to_string(),
                    password: password_val,
                })
                .await
            {
                Ok(_) => {
                    // 登录成功，清除表单
                    username.set(String::new());
                    password.set(String::new());
                    is_loading.set(false);
                    toast_store.show_success("登录成功".to_string());
                }
                Err(e) => {
                    is_loading.set(false);
                    let message = format!("登录失败: {}", e);
                    error_message.set(message.clone());
                    toast_store.show_error(message);
                }
            }
        });
    };

    rsx! {
        div { class: "login-page",
            div { class: "login-container",
                div { class: "login-card",
                    h1 { class: "login-title", "登录" }

                    if !error_message().is_empty() {
                        div { class: "error-message",
                            "{error_message()}"
                        }
                    }

                    div { class: "login-form",
                        label { for: "username", "用户名" }
                        input {
                            r#type: "text",
                            id: "username",
                            placeholder: "请输入用户名",
                            value: "{username}",
                            oninput: move |e| username.set(e.value()),
                            disabled: is_loading()
                        }

                        label { for: "password", "密码" }
                        input {
                            r#type: "password",
                            id: "password",
                            placeholder: "请输入密码",
                            value: "{password}",
                            oninput: move |e| password.set(e.value()),
                            disabled: is_loading()
                        }

                        button {
                            class: "btn btn-primary btn-full",
                            disabled: is_loading(),
                            onclick: handle_login,
                            if is_loading() {
                                "登录中..."
                            } else {
                                "登录"
                            }
                        }
                    }

                    div { class: "login-footer",
                        p { "默认账户: username / password（建议通过环境变量覆盖）" }
                    }
                }
            }
        }
    }
}
