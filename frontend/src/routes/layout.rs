use dioxus::prelude::*;

/// 主布局组件
#[component]
pub fn Layout(children: Element) -> Element {
    let mut sidebar_open = use_signal(|| false);
    let _toggle_sidebar = move |_: dioxus::events::MouseEvent| {
        sidebar_open.set(!sidebar_open());
    };

    rsx! {
        div { class: "app-layout",
            // 侧边栏
            aside { class: format!("sidebar {}", if *sidebar_open.read() { "open" } else { "" }),
                div { class: "sidebar-header",
                    h1 { "Vansour" }
                    p { class: "sidebar-subtitle", "图片管理" }
                }
                nav { class: "sidebar-nav",
                    ul {
                        li {
                            button { class: "nav-item", "仪表板" }
                        }
                        li {
                            button { class: "nav-item active", "图片" }
                        }
                        li {
                            button { class: "nav-item", "回收站" }
                        }
                    }
                }
            }

            // 主内容区
            main { class: "main-content",
                {children}
            }
        }
    }
}
