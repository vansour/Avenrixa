use crate::store::toast_store::{ToastMessage, ToastStore, ToastType};
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

const TOAST_AUTO_DISMISS_MS: u32 = 3_000;

/// Toast 提示组件
#[component]
pub fn Toast() -> Element {
    let toast_store = use_context::<ToastStore>();
    let toasts_signal = toast_store.toasts();
    let toasts = toasts_signal.read();

    rsx! {
        div { class: "toast-container",
            {toasts.iter().map(|toast| {
                rsx! {
                    ToastItem {
                        key: "{toast.id}",
                        toast: toast.clone(),
                    }
                }
            })}
        }
    }
}

#[component]
fn ToastItem(toast: ToastMessage) -> Element {
    let toast_store = use_context::<ToastStore>();
    let toast_id = toast.id;

    use_future(move || {
        let toast_store = toast_store.clone();
        async move {
            TimeoutFuture::new(TOAST_AUTO_DISMISS_MS).await;
            toast_store.remove_toast(toast_id);
        }
    });

    let class_name = match toast.toast_type {
        ToastType::Success => "toast-message toast-success",
        ToastType::Error => "toast-message toast-error",
        ToastType::Info => "toast-message toast-info",
    };

    rsx! {
        div {
            class: "{class_name}",
            "{toast.message}"
        }
    }
}
