use dioxus::prelude::*;

/// Toast 提示组件
#[component]
pub fn Toast(#[props(default)] message: Option<String>, children: Element) -> Element {
    rsx! {
        div { class: "toast-container",
            if let Some(msg) = message {
                div { class: "toast-message toast-success", "{msg}" }
            }
        }
    }
}
