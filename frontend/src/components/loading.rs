use dioxus::prelude::*;

/// Loading 加载组件
#[component]
pub fn Loading(#[props(default)] message: Option<String>) -> Element {
    rsx! {
        div { class: "loading-container",
            div { class: "loading-spinner" }
            if let Some(msg) = message {
                p { class: "loading-message", "{msg}" }
            }
        }
    }
}
