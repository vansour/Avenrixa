use crate::app_context::{use_auth_service, use_auth_store, use_toast_store};
use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopPage {
    Upload,
    History,
    Api,
    Settings,
}

/// 导航栏组件
#[component]
pub fn NavBar(
    site_name: String,
    current_page: TopPage,
    #[props(default)] on_navigate: EventHandler<TopPage>,
) -> Element {
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

    let on_go_upload = {
        let on_navigate = on_navigate.clone();
        move |_| on_navigate.call(TopPage::Upload)
    };
    let on_go_history = {
        let on_navigate = on_navigate.clone();
        move |_| on_navigate.call(TopPage::History)
    };
    let on_go_api = {
        let on_navigate = on_navigate.clone();
        move |_| on_navigate.call(TopPage::Api)
    };
    let on_go_settings = {
        let on_navigate = on_navigate.clone();
        move |_| on_navigate.call(TopPage::Settings)
    };
    let is_admin = user
        .as_ref()
        .map(|u| u.role.eq_ignore_ascii_case("admin"))
        .unwrap_or(false);

    rsx! {
        nav { class: "navbar",
            div { class: "navbar-container",
                div { class: "navbar-start",
                    div { class: "navbar-brand",
                        a { href: "/",
                            span { class: "navbar-brand-mark", "VI" }
                            div { class: "navbar-brand-copy",
                                span { class: "navbar-brand-title", "{site_name}" }
                                span { class: "navbar-brand-subtitle", "Visual Archive Console" }
                            }
                        }
                    }

                    if user.is_some() {
                        div { class: "navbar-tabs",
                            button {
                                class: format!("nav-tab {}", if current_page == TopPage::Upload { "active" } else { "" }),
                                onclick: on_go_upload,
                                "上传"
                            }
                            button {
                                class: format!("nav-tab {}", if current_page == TopPage::History { "active" } else { "" }),
                                onclick: on_go_history,
                                "历史"
                            }
                            button {
                                class: format!("nav-tab {}", if current_page == TopPage::Api { "active" } else { "" }),
                                onclick: on_go_api,
                                "API"
                            }
                            if is_admin {
                                button {
                                    class: format!("nav-tab {}", if current_page == TopPage::Settings { "active" } else { "" }),
                                    onclick: on_go_settings,
                                    "设置"
                                }
                            }
                        }
                    }
                }

                div { class: "navbar-menu",
                    if let Some(user) = user {
                        div { class: "user-info",
                            span { class: "user-name", "{user.username}" }
                            span { class: "user-role", "{user.role}" }

                            button {
                                class: "btn btn-ghost",
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
