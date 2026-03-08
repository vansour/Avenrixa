use dioxus::prelude::*;

/// 登录页面组件
#[component]
pub fn LoginPage() -> Element {
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut error_message = use_signal(String::new);

    let handle_login = move |_| async move {
        if username().is_empty() || password().is_empty() {
            error_message.set("请输入用户名和密码".to_string());
            return;
        }

        is_loading.set(true);
        error_message.set(String::new());

        // TODO: 调用实际的登录 API
        // 模拟登录过程
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        is_loading.set(false);
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
                        label { "用户名" }
                        input {
                            r#type: "text",
                            placeholder: "请输入用户名",
                            value: "{username}",
                            oninput: move |e| username.set(e.value())
                        }

                        label { "密码" }
                        input {
                            r#type: "password",
                            placeholder: "请输入密码",
                            value: "{password}",
                            oninput: move |e| password.set(e.value())
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
                        p { "默认账户: admin / password" }
                    }
                }
            }
        }
    }
}
