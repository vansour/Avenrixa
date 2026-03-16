use crate::services::AdminService;
use crate::store::{AuthStore, ToastStore};
use crate::types::api::{AdminUserSummary, UserRole};
use dioxus::prelude::*;
use std::collections::HashMap;

use super::super::{
    PendingUserRoleChange, role_change_confirmation_plan, set_settings_action_error,
};

pub(super) enum RoleChangePlan {
    Error(String),
    Info(String),
    RequiresConfirmation(Box<PendingUserRoleChange>),
}

pub(super) fn plan_role_change(
    users: &[AdminUserSummary],
    role_drafts: &HashMap<String, UserRole>,
    user_id: &str,
) -> RoleChangePlan {
    let Some(current_user) = users.iter().find(|user| user.id == user_id) else {
        return RoleChangePlan::Error("未找到要更新的用户".to_string());
    };

    let next_role = role_drafts
        .get(user_id)
        .cloned()
        .unwrap_or(current_user.role);

    if next_role == current_user.role {
        return RoleChangePlan::Info(format!("{} 的角色未发生变化", current_user.email));
    }

    let email = current_user.email.clone();
    RoleChangePlan::RequiresConfirmation(Box::new(PendingUserRoleChange {
        user_id: user_id.to_string(),
        email: email.clone(),
        next_role,
        plan: role_change_confirmation_plan(&email, current_user.role, next_role),
    }))
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn update_user_role(
    admin_service: AdminService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    user_id: String,
    email: String,
    next_role: UserRole,
    mut users: Signal<Vec<AdminUserSummary>>,
    mut role_drafts: Signal<HashMap<String, UserRole>>,
    mut updating_user_id: Signal<Option<String>>,
    mut error_message: Signal<String>,
    mut success_message: Signal<String>,
) {
    updating_user_id.set(Some(user_id.clone()));
    error_message.set(String::new());
    success_message.set(String::new());

    match admin_service.update_user_role(&user_id, next_role).await {
        Ok(_) => {
            let mut next_users = users();
            if let Some(user) = next_users.iter_mut().find(|user| user.id == user_id) {
                user.role = next_role;
            }
            users.set(next_users);

            let mut drafts = role_drafts();
            drafts.insert(user_id.clone(), next_role);
            role_drafts.set(drafts);

            let message = format!("已将 {} 的角色更新为 {}", email, next_role.label());
            success_message.set(message.clone());
            toast_store.show_success(message);
        }
        Err(err) => {
            set_settings_action_error(
                &auth_store,
                &toast_store,
                error_message,
                &err,
                "更新用户角色失败",
            );
        }
    }

    updating_user_id.set(None);
}
