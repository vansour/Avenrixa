use dioxus::prelude::*;

/// Modal 对话框组件
#[component]
pub fn Modal(
    #[props(default)] title: String,
    children: Element,
    on_close: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "modal-overlay",
            div { class: "modal-content",
                h2 { class: "modal-title", "{title}" }
                div { class: "modal-close", onclick: move |_| on_close.call(()), "×" }
                {children}
            }
        }
    }
}
