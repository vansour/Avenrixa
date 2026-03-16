use crate::services::AdminService;
use crate::store::{AuthStore, ToastStore};
use crate::types::api::{AdminUserSummary, UserRole};
use dioxus::prelude::*;
use std::collections::HashMap;

use super::super::set_settings_load_error;

#[allow(clippy::let_underscore_future, clippy::too_many_arguments)]
pub(super) fn use_users_loader(
    admin_service: AdminService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    users: Signal<Vec<AdminUserSummary>>,
    role_drafts: Signal<HashMap<String, UserRole>>,
    error_message: Signal<String>,
    is_loading: Signal<bool>,
    reload_tick: Signal<u64>,
) {
    let _ = use_resource(move || {
        let _ = reload_tick();
        let admin_service = admin_service.clone();
        let auth_store = auth_store.clone();
        let toast_store = toast_store.clone();
        async move {
            load_users(
                admin_service,
                auth_store,
                toast_store,
                users,
                role_drafts,
                error_message,
                is_loading,
            )
            .await;
        }
    });
}

async fn load_users(
    admin_service: AdminService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    mut users: Signal<Vec<AdminUserSummary>>,
    mut role_drafts: Signal<HashMap<String, UserRole>>,
    mut error_message: Signal<String>,
    mut is_loading: Signal<bool>,
) {
    is_loading.set(true);
    error_message.set(String::new());

    match admin_service.get_users().await {
        Ok(result) => {
            let next_role_map = result
                .iter()
                .map(|user| (user.id.clone(), user.role))
                .collect();
            users.set(result);
            role_drafts.set(next_role_map);
        }
        Err(err) => {
            set_settings_load_error(
                &auth_store,
                &toast_store,
                error_message,
                &err,
                "加载用户列表失败",
            );
        }
    }

    is_loading.set(false);
}
