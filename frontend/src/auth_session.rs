use crate::store::{AuthStore, ToastStore};
use crate::types::errors::AppError;

const AUTH_SESSION_EXPIRED_MESSAGE: &str = "登录状态已失效，请重新登录";

pub fn auth_session_expired_message() -> String {
    AUTH_SESSION_EXPIRED_MESSAGE.to_string()
}

pub fn handle_auth_session_error(
    auth_store: &AuthStore,
    toast_store: &ToastStore,
    err: &AppError,
) -> bool {
    if !err.should_redirect_login() {
        return false;
    }

    let was_authenticated = auth_store.is_authenticated();
    auth_store.logout();
    if was_authenticated {
        toast_store.show_error(AUTH_SESSION_EXPIRED_MESSAGE.to_string());
    }
    true
}
