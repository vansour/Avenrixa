use crate::app_context::{use_auth_store, use_navigation_store};
use crate::store::DashboardPage;
use dioxus::prelude::*;

const AUTH_NAV_PAGES: [DashboardPage; 5] = [
    DashboardPage::Upload,
    DashboardPage::History,
    DashboardPage::Trash,
    DashboardPage::Api,
    DashboardPage::Settings,
];

/// 导航栏组件
#[component]
pub fn NavBar(site_name: String) -> Element {
    let auth_store = use_auth_store();
    let navigation_store = use_navigation_store();
    let brand_navigation_store = navigation_store.clone();
    let login_navigation_store = navigation_store.clone();
    let mut is_mobile_menu_open = use_signal(|| false);

    let is_authenticated = auth_store.is_authenticated();
    let current_page = navigation_store.current_page();

    let nav_panel_class = if is_mobile_menu_open() {
        "navbar-panel is-open"
    } else {
        "navbar-panel"
    };
    let toggle_class = if is_mobile_menu_open() {
        "navbar-toggle is-open"
    } else {
        "navbar-toggle"
    };

    rsx! {
        nav { class: "navbar",
            div { class: "navbar-container",
                button {
                    r#type: "button",
                    class: "navbar-brand",
                    onclick: move |_| {
                        brand_navigation_store.reset();
                        is_mobile_menu_open.set(false);
                    },
                    span { class: "navbar-brand-mark", "VI" }
                    span { class: "navbar-brand-title", "{site_name}" }
                }

                if is_authenticated {
                    div { class: "{nav_panel_class}",
                        div { class: "navbar-tabs",
                            {AUTH_NAV_PAGES.iter().copied().map(|page| {
                                let navigation_store = navigation_store.clone();
                                let mut close_menu = is_mobile_menu_open;
                                let class_name = if current_page == page {
                                    "nav-tab active"
                                } else {
                                    "nav-tab"
                                };

                                rsx! {
                                    button {
                                        key: "{page.label()}",
                                        r#type: "button",
                                        class: "{class_name}",
                                        onclick: move |_| {
                                            navigation_store.navigate(page);
                                            spawn(async move {
                                                close_menu.set(false);
                                            });
                                        },
                                        strong { class: "nav-tab-title", "{page.label()}" }
                                    }
                                }
                            })}
                        }
                    }

                    button {
                        r#type: "button",
                        class: "{toggle_class}",
                        onclick: move |_| is_mobile_menu_open.toggle(),
                        span { class: "navbar-toggle-bars",
                            span {}
                            span {}
                            span {}
                        }
                    }
                } else {
                    button {
                        r#type: "button",
                        class: "btn btn-primary navbar-login-btn",
                        onclick: move |_| login_navigation_store.reset(),
                        "登录"
                    }
                }
            }
        }
    }
}
