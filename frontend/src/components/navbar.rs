use crate::app_context::use_auth_store;
use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopPage {
    Upload,
    History,
    Trash,
    Api,
    Settings,
}

impl TopPage {
    fn label(self) -> &'static str {
        match self {
            TopPage::Upload => "上传中心",
            TopPage::History => "历史图库",
            TopPage::Trash => "回收站",
            TopPage::Api => "API 接入",
            TopPage::Settings => "系统设置",
        }
    }
}

const AUTH_NAV_PAGES: [TopPage; 5] = [
    TopPage::Upload,
    TopPage::History,
    TopPage::Trash,
    TopPage::Api,
    TopPage::Settings,
];

/// 导航栏组件
#[component]
pub fn NavBar(
    site_name: String,
    current_page: TopPage,
    #[props(default)] on_navigate: EventHandler<TopPage>,
) -> Element {
    let auth_store = use_auth_store();
    let mut is_mobile_menu_open = use_signal(|| false);

    let is_authenticated = auth_store.is_authenticated();

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
                div { class: "navbar-brand",
                    a { href: "/",
                        span { class: "navbar-brand-mark", "VI" }
                        span { class: "navbar-brand-title", "{site_name}" }
                    }
                }

                if is_authenticated {
                    div { class: "{nav_panel_class}",
                        div { class: "navbar-tabs",
                            for page in AUTH_NAV_PAGES {
                                NavTabItem {
                                    page,
                                    current_page,
                                    on_navigate: on_navigate.clone(),
                                    close_menu: is_mobile_menu_open,
                                }
                            }
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
                    a { href: "/",
                        class: "btn btn-primary navbar-login-btn",
                        "登录"
                    }
                }
            }
        }
    }
}

#[component]
fn NavTabItem(
    page: TopPage,
    current_page: TopPage,
    on_navigate: EventHandler<TopPage>,
    close_menu: Signal<bool>,
) -> Element {
    let class_name = if current_page == page {
        "nav-tab active"
    } else {
        "nav-tab"
    };

    rsx! {
        button {
            r#type: "button",
            class: "{class_name}",
            onclick: move |_| {
                close_menu.set(false);
                on_navigate.call(page);
            },
            strong { class: "nav-tab-title", "{page.label()}" }
        }
    }
}
