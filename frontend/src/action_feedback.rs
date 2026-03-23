use std::future::Future;

use dioxus::prelude::*;

use crate::store::ToastStore;

pub(crate) fn set_action_error(
    mut error_message: Signal<String>,
    toast_store: &ToastStore,
    message: impl Into<String>,
) {
    let message = message.into();
    error_message.set(message.clone());
    toast_store.show_error(message);
}

pub(crate) fn spawn_tracked_action<Fut>(
    mut is_pending: Signal<bool>,
    mut error_message: Signal<String>,
    task: Fut,
) where
    Fut: Future<Output = ()> + 'static,
{
    spawn(async move {
        is_pending.set(true);
        error_message.set(String::new());
        task.await;
        is_pending.set(false);
    });
}
