mod actions;
mod loaders;

use crate::app_context::{use_admin_service, use_auth_store, use_toast_store};
use crate::components::ConfirmationModal;
use crate::types::api::{AdminUserSummary, UserRole};
use dioxus::prelude::*;
use std::collections::HashMap;

use self::actions::{RoleChangePlan, plan_role_change, update_user_role};
use self::loaders::use_users_loader;
use super::super::view::UsersSettingsSection;
use super::PendingUserRoleChange;

#[component]
pub fn UsersSectionController() -> Element {
    let admin_service = use_admin_service();
    let auth_store = use_auth_store();
    let toast_store = use_toast_store();

    let users = use_signal(Vec::<AdminUserSummary>::new);
    let role_drafts = use_signal(HashMap::<String, UserRole>::new);
    let mut error_message = use_signal(String::new);
    let mut success_message = use_signal(String::new);
    let is_loading = use_signal(|| false);
    let mut reload_tick = use_signal(|| 0_u64);
    let updating_user_id = use_signal(|| None::<String>);
    let mut pending_role_change = use_signal(|| None::<PendingUserRoleChange>);

    use_users_loader(
        admin_service.clone(),
        auth_store.clone(),
        toast_store.clone(),
        users,
        role_drafts,
        error_message,
        is_loading,
        reload_tick,
    );

    let handle_refresh = move |_| {
        if is_loading() || updating_user_id().is_some() {
            return;
        }
        reload_tick.set(reload_tick().wrapping_add(1));
    };

    let toast_store_for_user_role = toast_store.clone();
    let auth_store_for_confirm_role = auth_store.clone();
    let admin_service_for_confirm_role = admin_service.clone();
    let toast_store_for_confirm_role = toast_store.clone();
    let handle_save_user_role = move |user_id: String| {
        if updating_user_id().is_some() {
            return;
        }

        let current_users = users();
        let current_role_drafts = role_drafts();
        match plan_role_change(&current_users, &current_role_drafts, &user_id) {
            RoleChangePlan::Error(message) => {
                error_message.set(message.clone());
                toast_store_for_user_role.show_error(message);
            }
            RoleChangePlan::Info(message) => {
                success_message.set(message.clone());
                toast_store_for_user_role.show_info(message);
            }
            RoleChangePlan::RequiresConfirmation(pending) => {
                pending_role_change.set(Some(*pending));
            }
        }
    };

    rsx! {
        UsersSettingsSection {
            users: users(),
            role_drafts,
            error_message: error_message(),
            success_message: success_message(),
            is_loading: is_loading(),
            updating_user_id: updating_user_id(),
            on_refresh: handle_refresh,
            on_save_role: handle_save_user_role,
        }

        if let Some(pending) = pending_role_change() {
            ConfirmationModal {
                title: pending.plan.title.clone(),
                summary: pending.plan.summary.clone(),
                consequences: pending.plan.consequences.clone(),
                confirm_label: pending.plan.confirm_label.clone(),
                cancel_label: pending.plan.cancel_label.clone(),
                tone: pending.plan.tone,
                confirm_phrase: pending.plan.confirm_phrase.clone(),
                confirm_hint: pending.plan.confirm_hint.clone(),
                is_submitting: updating_user_id().is_some(),
                on_close: move |_| pending_role_change.set(None),
                on_confirm: move |_| {
                    let user_id = pending.user_id.clone();
                    let email = pending.email.clone();
                    let next_role = pending.next_role;
                    pending_role_change.set(None);

                    let admin_service = admin_service_for_confirm_role.clone();
                    let auth_store = auth_store_for_confirm_role.clone();
                    let toast_store = toast_store_for_confirm_role.clone();
                    spawn(async move {
                        update_user_role(
                            admin_service,
                            auth_store,
                            toast_store,
                            user_id,
                            email,
                            next_role,
                            users,
                            role_drafts,
                            updating_user_id,
                            error_message,
                            success_message,
                        )
                        .await;
                    });
                },
            }
        }
    }
}
