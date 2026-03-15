use crate::store::{AuthStore, ToastStore};
use crate::types::errors::AppError;
use dioxus::prelude::*;

use super::super::{handle_settings_auth_error, settings_auth_expired_message};

pub(crate) fn set_settings_load_error(
    auth_store: &AuthStore,
    toast_store: &ToastStore,
    mut error_message: Signal<String>,
    err: &AppError,
    prefix: &str,
) {
    if handle_settings_auth_error(auth_store, toast_store, err) {
        error_message.set(settings_auth_expired_message());
    } else {
        error_message.set(format!("{prefix}: {err}"));
    }
}

pub(crate) fn set_settings_action_error(
    auth_store: &AuthStore,
    toast_store: &ToastStore,
    mut error_message: Signal<String>,
    err: &AppError,
    prefix: &str,
) {
    if handle_settings_auth_error(auth_store, toast_store, err) {
        error_message.set(settings_auth_expired_message());
    } else {
        let message = format!("{prefix}: {err}");
        error_message.set(message.clone());
        toast_store.show_error(message);
    }
}
