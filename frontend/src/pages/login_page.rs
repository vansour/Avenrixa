use crate::app_context::{use_auth_service, use_toast_store};
use crate::types::api::{LoginRequest, RegisterRequest};
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum LoginMode {
    Login,
    Register,
    RequestReset,
    ConfirmReset,
    ConfirmEmailVerification,
}

#[cfg(target_arch = "wasm32")]
fn read_query_param(name: &str) -> Option<String> {
    let search = web_sys::window()
        .and_then(|window| window.location().search().ok())
        .unwrap_or_default();
    let search = search.trim_start_matches('?');
    if search.is_empty() {
        return None;
    }

    for pair in search.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        if key == name {
            return urlencoding::decode(value)
                .ok()
                .map(|value| value.into_owned());
        }
    }

    None
}

#[cfg(not(target_arch = "wasm32"))]
fn read_query_param(_name: &str) -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
fn clear_auth_query_from_location() {
    use wasm_bindgen::JsValue;

    if let Some(window) = web_sys::window()
        && let Ok(history) = window.history()
    {
        let pathname = window.location().pathname().ok().unwrap_or_default();
        let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&pathname));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn clear_auth_query_from_location() {}

fn initial_mode() -> LoginMode {
    match read_query_param("mode").as_deref() {
        Some("verify-email") if read_query_param("token").is_some() => {
            LoginMode::ConfirmEmailVerification
        }
        _ if read_query_param("token").is_some() => LoginMode::ConfirmReset,
        _ => LoginMode::Login,
    }
}

/// 登录页面组件
#[component]
pub fn LoginPage(mail_enabled: bool) -> Element {
    let auth_service = use_auth_service();
    let toast_store = use_toast_store();

    let initial_token = read_query_param("token");
    let mut mode = use_signal(initial_mode);

    let mut login_email = use_signal(String::new);
    let mut register_email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut register_confirm_password = use_signal(String::new);
    let mut reset_email = use_signal(String::new);
    let mut reset_token = use_signal(|| initial_token.clone().unwrap_or_default());
    let mut verification_token = use_signal(|| initial_token.clone().unwrap_or_default());
    let mut new_password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut error_message = use_signal(String::new);

    let show_login = mode() == LoginMode::Login;
    let show_register = mode() == LoginMode::Register;
    let show_request_reset = mode() == LoginMode::RequestReset;
    let show_confirm_reset = mode() == LoginMode::ConfirmReset;
    let show_confirm_email_verification = mode() == LoginMode::ConfirmEmailVerification;

    use_effect(move || {
        if mail_enabled {
            return;
        }

        if matches!(mode(), LoginMode::Register | LoginMode::RequestReset) {
            error_message.set(String::new());
            mode.set(LoginMode::Login);
        }
    });

    let auth_service_for_login = auth_service.clone();
    let toast_store_for_login = toast_store.clone();
    let handle_login = move |_| {
        let auth_service_clone = auth_service_for_login.clone();
        let toast_store = toast_store_for_login.clone();
        spawn(async move {
            let email_val = login_email();
            let password_val = password();

            if email_val.trim().is_empty() || password_val.trim().is_empty() {
                let message = "请输入邮箱和密码".to_string();
                error_message.set(message.clone());
                toast_store.show_error(message);
                return;
            }

            is_loading.set(true);
            error_message.set(String::new());

            match auth_service_clone
                .login(LoginRequest {
                    email: email_val.trim().to_string(),
                    password: password_val,
                })
                .await
            {
                Ok(_) => {
                    login_email.set(String::new());
                    password.set(String::new());
                    toast_store.show_success("登录成功".to_string());
                }
                Err(error) => {
                    let message = format!("登录失败: {}", error);
                    error_message.set(message.clone());
                    toast_store.show_error(message);
                }
            }

            is_loading.set(false);
        });
    };

    let auth_service_for_register = auth_service.clone();
    let toast_store_for_register = toast_store.clone();
    let handle_register = move |_| {
        let auth_service = auth_service_for_register.clone();
        let toast_store = toast_store_for_register.clone();
        spawn(async move {
            let email_val = register_email();
            let password_val = password();
            let confirm_val = register_confirm_password();

            if email_val.trim().is_empty() || password_val.trim().is_empty() {
                let message = "请填写邮箱和密码".to_string();
                error_message.set(message.clone());
                toast_store.show_error(message);
                return;
            }
            if password_val != confirm_val {
                let message = "两次输入的密码不一致".to_string();
                error_message.set(message.clone());
                toast_store.show_error(message);
                return;
            }

            is_loading.set(true);
            error_message.set(String::new());

            match auth_service
                .register(RegisterRequest {
                    email: email_val.trim().to_string(),
                    password: password_val,
                })
                .await
            {
                Ok(_) => {
                    toast_store.show_success("注册成功，请查收邮箱完成验证".to_string());
                    login_email.set(String::new());
                    register_email.set(String::new());
                    password.set(String::new());
                    register_confirm_password.set(String::new());
                    mode.set(LoginMode::Login);
                }
                Err(error) => {
                    let message = format!("注册失败: {}", error);
                    error_message.set(message.clone());
                    toast_store.show_error(message);
                }
            }

            is_loading.set(false);
        });
    };

    let auth_service_for_request_reset = auth_service.clone();
    let toast_store_for_request_reset = toast_store.clone();
    let handle_request_reset = move |_| {
        let auth_service = auth_service_for_request_reset.clone();
        let toast_store = toast_store_for_request_reset.clone();
        spawn(async move {
            let email = reset_email();
            if email.trim().is_empty() {
                let message = "请输入邮箱".to_string();
                error_message.set(message.clone());
                toast_store.show_error(message);
                return;
            }

            is_loading.set(true);
            error_message.set(String::new());

            match auth_service
                .request_password_reset(email.trim().to_string())
                .await
            {
                Ok(_) => {
                    toast_store.show_success("如果账号已配置找回邮箱，重置邮件已发送".to_string());
                    reset_email.set(String::new());
                    mode.set(LoginMode::Login);
                }
                Err(error) => {
                    let message = format!("发送重置邮件失败: {}", error);
                    error_message.set(message.clone());
                    toast_store.show_error(message);
                }
            }

            is_loading.set(false);
        });
    };

    let auth_service_for_confirm_reset = auth_service.clone();
    let toast_store_for_confirm_reset = toast_store.clone();
    let handle_confirm_reset = move |_| {
        let auth_service = auth_service_for_confirm_reset.clone();
        let toast_store = toast_store_for_confirm_reset.clone();
        spawn(async move {
            if reset_token().trim().is_empty() {
                let message = "重置令牌不能为空".to_string();
                error_message.set(message.clone());
                toast_store.show_error(message);
                return;
            }
            if new_password().trim().is_empty() {
                let message = "请输入新密码".to_string();
                error_message.set(message.clone());
                toast_store.show_error(message);
                return;
            }
            if new_password() != confirm_password() {
                let message = "两次输入的新密码不一致".to_string();
                error_message.set(message.clone());
                toast_store.show_error(message);
                return;
            }

            is_loading.set(true);
            error_message.set(String::new());

            match auth_service
                .confirm_password_reset(reset_token().trim().to_string(), new_password())
                .await
            {
                Ok(_) => {
                    toast_store.show_success("密码已重置，请使用新密码登录".to_string());
                    clear_auth_query_from_location();
                    reset_token.set(String::new());
                    new_password.set(String::new());
                    confirm_password.set(String::new());
                    mode.set(LoginMode::Login);
                }
                Err(error) => {
                    let message = format!("重置密码失败: {}", error);
                    error_message.set(message.clone());
                    toast_store.show_error(message);
                }
            }

            is_loading.set(false);
        });
    };

    let auth_service_for_confirm_verification = auth_service.clone();
    let toast_store_for_confirm_verification = toast_store.clone();
    let handle_confirm_verification = move |_| {
        let auth_service = auth_service_for_confirm_verification.clone();
        let toast_store = toast_store_for_confirm_verification.clone();
        spawn(async move {
            if verification_token().trim().is_empty() {
                let message = "验证令牌不能为空".to_string();
                error_message.set(message.clone());
                toast_store.show_error(message);
                return;
            }

            is_loading.set(true);
            error_message.set(String::new());

            match auth_service
                .confirm_email_verification(verification_token().trim().to_string())
                .await
            {
                Ok(_) => {
                    toast_store.show_success("邮箱验证成功，请使用新账号登录".to_string());
                    clear_auth_query_from_location();
                    verification_token.set(String::new());
                    mode.set(LoginMode::Login);
                }
                Err(error) => {
                    let message = format!("邮箱验证失败: {}", error);
                    error_message.set(message.clone());
                    toast_store.show_error(message);
                }
            }

            is_loading.set(false);
        });
    };

    let switch_to_login = move |_| {
        error_message.set(String::new());
        mode.set(LoginMode::Login);
    };
    let switch_to_register = move |_| {
        error_message.set(String::new());
        mode.set(LoginMode::Register);
    };
    let switch_to_request_reset = move |_| {
        error_message.set(String::new());
        mode.set(LoginMode::RequestReset);
    };

    rsx! {
        div { class: "login-page",
            div { class: "login-container",
                div { class: "login-card",
                    h1 { class: "login-title",
                        if show_request_reset {
                            "重置密码"
                        } else if show_confirm_email_verification {
                            "验证邮箱"
                        } else if show_register {
                            "创建账号"
                        } else if show_confirm_reset {
                            "设置新密码"
                        } else {
                            "登录控制台"
                        }
                    }
                    p { class: "login-subtitle",
                        if show_request_reset {
                            "输入邮箱，我们会向已配置的地址发送重置链接"
                        } else if show_confirm_email_verification {
                            "验证邮箱后即可使用新账号登录"
                        } else if show_register {
                            "注册后需要完成邮箱验证"
                        } else if show_confirm_reset {
                            "输入新密码以完成重置"
                        } else if mail_enabled {
                            "管理图片资产与访问权限"
                        } else {
                            "当前站点未启用邮件能力，仅支持已有账号直接登录"
                        }
                    }

                    if !error_message().is_empty() {
                        div { class: "error-banner", "{error_message()}" }
                    }

                    if show_login {
                        div { class: "login-form",
                            label { for: "login-email", "邮箱" }
                            input {
                                r#type: "email",
                                id: "login-email",
                                placeholder: "请输入邮箱地址",
                                value: "{login_email}",
                                oninput: move |e| login_email.set(e.value()),
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
                                if is_loading() { "登录中..." } else { "登录" }
                            }
                        }
                    }

                    if show_register {
                        div { class: "login-form",
                            label { for: "register-email", "邮箱" }
                            input {
                                r#type: "email",
                                id: "register-email",
                                placeholder: "请输入邮箱地址",
                                value: "{register_email}",
                                oninput: move |e| register_email.set(e.value()),
                                disabled: is_loading()
                            }

                            label { for: "register-password", "密码" }
                            input {
                                r#type: "password",
                                id: "register-password",
                                placeholder: "请输入密码",
                                value: "{password}",
                                oninput: move |e| password.set(e.value()),
                                disabled: is_loading()
                            }

                            label { for: "register-confirm-password", "确认密码" }
                            input {
                                r#type: "password",
                                id: "register-confirm-password",
                                placeholder: "请再次输入密码",
                                value: "{register_confirm_password}",
                                oninput: move |e| register_confirm_password.set(e.value()),
                                disabled: is_loading()
                            }

                            button {
                                class: "btn btn-primary btn-full",
                                disabled: is_loading(),
                                onclick: handle_register,
                                if is_loading() { "注册中..." } else { "注册并发送验证邮件" }
                            }
                        }
                    }

                    if show_request_reset {
                        div { class: "login-form",
                            label { for: "reset-email", "邮箱" }
                            input {
                                r#type: "email",
                                id: "reset-email",
                                placeholder: "请输入邮箱地址",
                                value: "{reset_email}",
                                oninput: move |e| reset_email.set(e.value()),
                                disabled: is_loading()
                            }

                            button {
                                class: "btn btn-primary btn-full",
                                disabled: is_loading(),
                                onclick: handle_request_reset,
                                if is_loading() { "发送中..." } else { "发送重置邮件" }
                            }
                        }
                    }

                    if show_confirm_reset {
                        div { class: "login-form",
                            label { for: "reset-token", "重置令牌" }
                            input {
                                r#type: "text",
                                id: "reset-token",
                                placeholder: "请输入邮件中的重置令牌",
                                value: "{reset_token}",
                                oninput: move |e| reset_token.set(e.value()),
                                disabled: is_loading()
                            }

                            label { for: "new-password", "新密码" }
                            input {
                                r#type: "password",
                                id: "new-password",
                                placeholder: "请输入新密码",
                                value: "{new_password}",
                                oninput: move |e| new_password.set(e.value()),
                                disabled: is_loading()
                            }

                            label { for: "confirm-password", "确认新密码" }
                            input {
                                r#type: "password",
                                id: "confirm-password",
                                placeholder: "请再次输入新密码",
                                value: "{confirm_password}",
                                oninput: move |e| confirm_password.set(e.value()),
                                disabled: is_loading()
                            }

                            button {
                                class: "btn btn-primary btn-full",
                                disabled: is_loading(),
                                onclick: handle_confirm_reset,
                                if is_loading() { "提交中..." } else { "重置密码" }
                            }
                        }
                    }

                    if show_confirm_email_verification {
                        div { class: "login-form",
                            label { for: "verification-token", "验证令牌" }
                            input {
                                r#type: "text",
                                id: "verification-token",
                                placeholder: "请输入邮件中的验证令牌",
                                value: "{verification_token}",
                                oninput: move |e| verification_token.set(e.value()),
                                disabled: is_loading()
                            }

                            button {
                                class: "btn btn-primary btn-full",
                                disabled: is_loading(),
                                onclick: handle_confirm_verification,
                                if is_loading() { "验证中..." } else { "完成邮箱验证" }
                            }
                        }
                    }

                    div { class: "login-footer",
                        p {
                            class: "login-tip",
                            if show_register {
                                "注册后需要点击邮件中的验证链接激活账号。"
                            } else if show_confirm_email_verification {
                                "验证成功后即可返回登录。"
                            } else if mail_enabled {
                                "如果你还没有账号，可以先完成公开注册。"
                            } else {
                                "注册和密码找回入口会在邮件能力启用后开放。"
                            }
                        }
                        if show_login {
                            if mail_enabled {
                                div { class: "login-form",
                                    button {
                                        class: "btn btn-ghost btn-full",
                                        disabled: is_loading(),
                                        onclick: switch_to_register,
                                        "注册新账号"
                                    }
                                    button {
                                        class: "btn btn-ghost btn-full",
                                        disabled: is_loading(),
                                        onclick: switch_to_request_reset,
                                        "忘记密码"
                                    }
                                }
                            }
                        } else {
                            button {
                                class: "btn btn-ghost btn-full",
                                disabled: is_loading(),
                                onclick: switch_to_login,
                                "返回登录"
                            }
                        }
                    }
                }
            }
        }
    }
}
