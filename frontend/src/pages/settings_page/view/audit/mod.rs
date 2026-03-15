mod details;
mod logic;

use crate::types::api::AuditLog;
use dioxus::prelude::*;

use self::logic::{
    audit_actor_label, audit_category_class, audit_category_label, audit_risk_label, audit_summary,
    audit_target_type_label, audit_title,
};
use super::shared::{format_json_details, format_timestamp, optional_short_id, page_window};

#[component]
pub fn AuditSettingsSection(
    logs: Vec<AuditLog>,
    total: i64,
    current_page: i32,
    page_size: i32,
    has_next: bool,
    error_message: String,
    is_loading: bool,
    #[props(default)] on_refresh: EventHandler<MouseEvent>,
    #[props(default)] on_prev_page: EventHandler<MouseEvent>,
    #[props(default)] on_next_page: EventHandler<MouseEvent>,
    #[props(default)] on_page_size_change: EventHandler<String>,
) -> Element {
    let (start, end) = page_window(current_page, page_size, total);

    rsx! {
        div { class: "settings-stack",
            div { class: "settings-list-toolbar",
                div { class: "settings-toolbar-meta",
                    span { class: "stat-pill", "日志总数 {total}" }
                    span { class: "stat-pill", "当前第 {current_page} 页" }
                    if total > 0 {
                        span { class: "stat-pill stat-pill-active", "显示 {start}-{end}" }
                    }
                }
                div { class: "settings-list-actions",
                    label { class: "page-size-control",
                        span { "每页" }
                        select {
                            class: "page-size-select",
                            value: "{page_size}",
                            disabled: is_loading,
                            onchange: move |event| on_page_size_change.call(event.value()),
                            option { value: "20", "20" }
                            option { value: "50", "50" }
                            option { value: "100", "100" }
                        }
                        span { "条" }
                    }
                    button {
                        class: "btn",
                        disabled: is_loading,
                        onclick: move |event| on_refresh.call(event),
                        if is_loading { "刷新中..." } else { "刷新日志" }
                    }
                    div { class: "page-actions",
                        button {
                            class: "btn",
                            disabled: is_loading || current_page <= 1,
                            onclick: move |event| on_prev_page.call(event),
                            "上一页"
                        }
                        button {
                            class: "btn",
                            disabled: is_loading || !has_next,
                            onclick: move |event| on_next_page.call(event),
                            "下一页"
                        }
                    }
                }
            }

            if !error_message.is_empty() {
                div { class: "error-banner", "{error_message}" }
            }

            if is_loading && logs.is_empty() {
                div { class: "settings-placeholder settings-placeholder-compact",
                    h3 { "正在加载审计日志" }
                }
            } else if logs.is_empty() {
                div { class: "settings-placeholder settings-placeholder-compact",
                    h3 { "暂无审计记录" }
                }
            } else {
                div { class: "settings-log-list",
                    for log in logs {
                        {render_audit_log_card(log)}
                    }
                }
            }
        }
    }
}

fn render_audit_log_card(log: AuditLog) -> Element {
    let audit_title = audit_title(&log);
    let audit_summary = audit_summary(&log);
    let audit_category = audit_category_label(&log);
    let audit_category_class = audit_category_class(&log);
    let audit_risk = audit_risk_label(&log);
    let target_type = audit_target_type_label(&log.target_type);
    let actor = audit_actor_label(&log);

    rsx! {
        article { class: "settings-log-card",
            div { class: "settings-log-head",
                div { class: "settings-log-title",
                    h3 { "{audit_title}" }
                    p { class: "settings-log-meta",
                        "{format_timestamp(log.created_at)} · 目标 {target_type}"
                    }
                    p { class: "settings-log-summary", "{audit_summary}" }
                }
                span { class: format!("settings-log-tag {}", audit_category_class), "{audit_category}" }
            }

            div { class: "settings-log-meta-grid",
                span { class: format!("settings-log-chip {}", audit_category_class), "{audit_risk}" }
                span { class: "settings-log-chip", "{log.action}" }
                span { class: "settings-log-chip", "操作者 {actor}" }
                span { class: "settings-log-chip", "目标ID {optional_short_id(log.target_id.as_deref())}" }
                span { class: "settings-log-chip", {"IP ".to_string() + &log.ip_address.clone().unwrap_or_else(|| "未知".to_string())} }
            }

            if let Some(details) = &log.details {
                pre { class: "settings-code-block", "{format_json_details(details)}" }
            }
        }
    }
}
