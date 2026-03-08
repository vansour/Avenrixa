use crate::store::toast_store::ToastStore;
use dioxus::prelude::*;

/// Toast 提示组件
#[component]
pub fn Toast() -> Element {
    let toast_store = use_context::<ToastStore>();
    let toasts_signal = toast_store.toasts();
    let toasts = toasts_signal.read();

    rsx! {
        div { class: "toast-container",
            {toasts.iter().map(|toast| {
                let class_name = match toast.toast_type {
                    crate::store::toast_store::ToastType::Success => "toast-message toast-success",
                    crate::store::toast_store::ToastType::Error => "toast-message toast-error",
                    crate::store::toast_store::ToastType::Info => "toast-message toast-info",
                };
                rsx! {
                    div {
                        key: "{toast.id}",
                        class: "{class_name}",
                        "{toast.message}"
                    }
                }
            })}
        }
    }
}
