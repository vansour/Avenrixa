use dioxus::prelude::*;

/// 应用程序入口组件
#[component]
pub fn App() -> Element {
    rsx! {
        div { class: "app",
            "Vansour Image"
        }
    }
}
