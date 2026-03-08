use crate::app_context::{use_auth_service, use_auth_store, use_toast_store};
use dioxus::prelude::*;

/// 导航栏组件
#[component]
pub fn NavBar() -> Element {
    let auth_service = use_auth_service();
    let auth_store = use_auth_store();
    let toast_store = use_toast_store();
    let mut is_logging_out = use_signal(|| false);

    let user = auth_store.user();

    let handle_logout = move |_| {
        if is_logging_out() {
            return;
        }

        let auth_service = auth_service.clone();
        let auth_store = auth_store.clone();
        let toast_store = toast_store.clone();
        spawn(async move {
            is_logging_out.set(true);

            match auth_service.logout().await {
                Ok(_) => {
                    toast_store.show_success("已登出".to_string());
                }
                Err(e) => {
                    if e.should_redirect_login() {
                        auth_store.logout();
                    }
                    toast_store.show_error(format!("登出失败: {}", e));
                    eprintln!("登出失败: {}", e);
                }
            }

            is_logging_out.set(false);
        });
    };

    rsx! {
        nav { class: "navbar",
            div { class: "navbar-container",
                div { class: "navbar-brand",
                    a { href: "/",
                        img { src: "/favicon.ico", alt: "Vansour Image" }
                        span { "Vansour Image" }
                    }
                }

                div { class: "navbar-menu",
                    if let Some(user) = user {
                        div { class: "user-info",
                            span { class: "user-name", "{user.username}" }
                            span { class: "user-role", "({user.role})" }

                            button {
                                class: "btn btn-logout",
                                disabled: is_logging_out(),
                                onclick: handle_logout,
                                if is_logging_out() {
                                    "登出中..."
                                } else {
                                    "登出"
                                }
                            }
                        }
                    } else {
                        a { href: "/",
                            class: "btn btn-primary",
                            "登录"
                        }
                    }
                }
            }
        }
    }
}
