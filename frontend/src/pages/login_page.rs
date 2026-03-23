mod controller;

use dioxus::prelude::*;

use controller::use_login_page_controller;

/// 登录页面组件
#[component]
pub fn LoginPage(mail_enabled: bool) -> Element {
    let controller = use_login_page_controller(mail_enabled);

    let show_login = controller.show_login();
    let show_register = controller.show_register();
    let show_request_reset = controller.show_request_reset();
    let show_confirm_reset = controller.show_confirm_reset();
    let show_confirm_email_verification = controller.show_confirm_email_verification();
    let is_loading = controller.is_loading();
    let error_message = controller.error_message();

    let mut login_email = controller.login_email;
    let mut register_email = controller.register_email;
    let mut password = controller.password;
    let mut register_confirm_password = controller.register_confirm_password;
    let mut reset_email = controller.reset_email;
    let mut reset_token = controller.reset_token;
    let mut verification_token = controller.verification_token;
    let mut new_password = controller.new_password;
    let mut confirm_password = controller.confirm_password;

    let controller_for_login = controller.clone();
    let controller_for_register = controller.clone();
    let controller_for_request_reset = controller.clone();
    let controller_for_confirm_reset = controller.clone();
    let controller_for_confirm_verification = controller.clone();
    let controller_for_switch_login = controller.clone();
    let controller_for_switch_register = controller.clone();
    let controller_for_switch_request_reset = controller.clone();

    rsx! {
        div { class: "login-page",
            div { class: "login-container",
                div { class: "login-card",
                    h1 { class: "login-title", "{controller.title()}" }
                    p { class: "login-subtitle", "{controller.subtitle()}" }

                    if !error_message.is_empty() {
                        div { class: "error-banner", "{error_message}" }
                    }

                    if show_login {
                        div { class: "login-form",
                            label { for: "login-email", "邮箱" }
                            input {
                                r#type: "email",
                                id: "login-email",
                                placeholder: "请输入邮箱地址",
                                value: "{login_email}",
                                oninput: move |event| login_email.set(event.value()),
                                disabled: is_loading,
                            }

                            label { for: "password", "密码" }
                            input {
                                r#type: "password",
                                id: "password",
                                placeholder: "请输入密码",
                                value: "{password}",
                                oninput: move |event| password.set(event.value()),
                                disabled: is_loading,
                            }

                            button {
                                class: "btn btn-primary btn-full",
                                disabled: is_loading,
                                onclick: move |_| controller_for_login.submit_login(),
                                if is_loading { "登录中..." } else { "登录" }
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
                                oninput: move |event| register_email.set(event.value()),
                                disabled: is_loading,
                            }

                            label { for: "register-password", "密码" }
                            input {
                                r#type: "password",
                                id: "register-password",
                                placeholder: "请输入密码",
                                value: "{password}",
                                oninput: move |event| password.set(event.value()),
                                disabled: is_loading,
                            }

                            label { for: "register-confirm-password", "确认密码" }
                            input {
                                r#type: "password",
                                id: "register-confirm-password",
                                placeholder: "请再次输入密码",
                                value: "{register_confirm_password}",
                                oninput: move |event| register_confirm_password.set(event.value()),
                                disabled: is_loading,
                            }

                            button {
                                class: "btn btn-primary btn-full",
                                disabled: is_loading,
                                onclick: move |_| controller_for_register.submit_register(),
                                if is_loading { "注册中..." } else { "注册并发送验证邮件" }
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
                                oninput: move |event| reset_email.set(event.value()),
                                disabled: is_loading,
                            }

                            button {
                                class: "btn btn-primary btn-full",
                                disabled: is_loading,
                                onclick: move |_| controller_for_request_reset.submit_request_reset(),
                                if is_loading { "发送中..." } else { "发送重置邮件" }
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
                                oninput: move |event| reset_token.set(event.value()),
                                disabled: is_loading,
                            }

                            label { for: "new-password", "新密码" }
                            input {
                                r#type: "password",
                                id: "new-password",
                                placeholder: "请输入新密码",
                                value: "{new_password}",
                                oninput: move |event| new_password.set(event.value()),
                                disabled: is_loading,
                            }

                            label { for: "confirm-password", "确认新密码" }
                            input {
                                r#type: "password",
                                id: "confirm-password",
                                placeholder: "请再次输入新密码",
                                value: "{confirm_password}",
                                oninput: move |event| confirm_password.set(event.value()),
                                disabled: is_loading,
                            }

                            button {
                                class: "btn btn-primary btn-full",
                                disabled: is_loading,
                                onclick: move |_| controller_for_confirm_reset.submit_confirm_reset(),
                                if is_loading { "提交中..." } else { "重置密码" }
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
                                oninput: move |event| verification_token.set(event.value()),
                                disabled: is_loading,
                            }

                            button {
                                class: "btn btn-primary btn-full",
                                disabled: is_loading,
                                onclick: move |_| {
                                    controller_for_confirm_verification
                                        .submit_confirm_email_verification()
                                },
                                if is_loading { "验证中..." } else { "完成邮箱验证" }
                            }
                        }
                    }

                    div { class: "login-footer",
                        p { class: "login-tip", "{controller.footer_tip()}" }
                        if show_login {
                            if mail_enabled {
                                div { class: "login-form",
                                    button {
                                        class: "btn btn-ghost btn-full",
                                        disabled: is_loading,
                                        onclick: move |_| controller_for_switch_register.switch_to_register(),
                                        "注册新账号"
                                    }
                                    button {
                                        class: "btn btn-ghost btn-full",
                                        disabled: is_loading,
                                        onclick: move |_| {
                                            controller_for_switch_request_reset.switch_to_request_reset()
                                        },
                                        "忘记密码"
                                    }
                                }
                            }
                        } else {
                            button {
                                class: "btn btn-ghost btn-full",
                                disabled: is_loading,
                                onclick: move |_| controller_for_switch_login.switch_to_login(),
                                "返回登录"
                            }
                        }
                    }
                }
            }
        }
    }
}
