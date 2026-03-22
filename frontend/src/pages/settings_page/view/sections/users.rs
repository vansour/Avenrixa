use crate::types::api::{AdminUserSummary, UserRole};
use dioxus::prelude::*;
use std::collections::HashMap;

use super::super::shared::{format_timestamp, short_identifier};

#[component]
pub fn UsersSettingsSection(
    users: Vec<AdminUserSummary>,
    role_drafts: Signal<HashMap<String, UserRole>>,
    error_message: String,
    success_message: String,
    is_loading: bool,
    updating_user_id: Option<String>,
    #[props(default)] on_refresh: EventHandler<MouseEvent>,
    #[props(default)] on_save_role: EventHandler<String>,
) -> Element {
    let is_updating_any = updating_user_id.is_some();

    rsx! {
        div { class: "settings-stack",
            div { class: "settings-list-toolbar",
                div { class: "settings-toolbar-meta",
                    span { class: "stat-pill", "账户 {users.len()} 个" }
                }
                div { class: "settings-inline-actions",
                    button {
                        class: "btn",
                        disabled: is_loading || is_updating_any,
                        onclick: move |event| on_refresh.call(event),
                        if is_loading { "刷新中..." } else { "刷新列表" }
                    }
                }
            }

            if !error_message.is_empty() {
                div { class: "error-banner", "{error_message}" }
            }

            if !success_message.is_empty() {
                div { class: "settings-banner settings-banner-success", "{success_message}" }
            }

            if is_loading && users.is_empty() {
                div { class: "settings-placeholder settings-placeholder-compact",
                    h3 { "正在加载用户列表" }
                }
            } else if users.is_empty() {
                div { class: "settings-placeholder settings-placeholder-compact",
                    h3 { "暂时没有可展示的用户" }
                }
            } else {
                div { class: "settings-entity-list",
                    {users.into_iter().map(|user| {
                        let user_id_for_select = user.id.clone();
                        let user_id_for_save = user.id.clone();
                        let current_role = role_drafts()
                            .get(&user.id)
                            .cloned()
                            .unwrap_or(user.role);
                        let is_row_updating = updating_user_id.as_deref() == Some(user.id.as_str());
                        rsx! {
                            article { class: "settings-entity-card",
                                div { class: "settings-entity-main",
                                    div { class: "settings-entity-copy",
                                        div { class: "settings-entity-title",
                                            h3 { "{user.email}" }
                                            span {
                                                class: format!(
                                                    "settings-role-badge {}",
                                                    user.role.surface_class()
                                                ),
                                                {user.role.label()}
                                            }
                                        }
                                        p { class: "settings-entity-meta",
                                            "用户 ID {short_identifier(&user.id)} · 创建于 {format_timestamp(user.created_at)}"
                                        }
                                    }

                                    div { class: "settings-entity-controls",
                                        label { class: "settings-field settings-inline-field",
                                            span { "角色" }
                                            select {
                                                value: "{current_role.as_str()}",
                                                disabled: is_loading || is_updating_any,
                                                onchange: move |event| {
                                                    let mut drafts = role_drafts();
                                                    drafts.insert(
                                                        user_id_for_select.clone(),
                                                        UserRole::parse(&event.value()),
                                                    );
                                                    role_drafts.set(drafts);
                                                },
                                                option { value: UserRole::Admin.as_str(), "admin" }
                                                option { value: UserRole::User.as_str(), "user" }
                                            }
                                        }
                                        button {
                                            class: "btn btn-primary",
                                            disabled: is_loading || is_updating_any,
                                            onclick: move |_| on_save_role.call(user_id_for_save.clone()),
                                            if is_row_updating { "保存中..." } else { "保存角色" }
                                        }
                                    }
                                }
                            }
                        }
                    })}
                }
            }
        }
    }
}
