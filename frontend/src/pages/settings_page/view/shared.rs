use crate::types::api::{BackupDatabaseFamily, BackupSemantics, ComponentStatus};
use chrono::{DateTime, Utc};
use dioxus::prelude::*;

use super::section::SettingsSection;

pub(super) fn render_placeholder_section(section: SettingsSection) -> Element {
    rsx! {
        div { class: "settings-placeholder settings-placeholder-compact",
            h3 { "{section.title()}" }
        }
    }
}

pub(super) fn render_component_status_card(title: &str, status: &ComponentStatus) -> Element {
    rsx! {
        article { class: format!("settings-status-card {}", status.status.surface_class()),
            p { class: "settings-summary-label", "{title}" }
            h3 { "{status.status.label()}" }
            if let Some(message) = &status.message {
                p { class: "settings-status-message", "{message}" }
            } else {
                p { class: "settings-status-message", "运行正常" }
            }
        }
    }
}

pub(super) fn render_metric_card(title: &str, value: String) -> Element {
    rsx! {
        article { class: "settings-metric-card",
            p { class: "settings-summary-label", "{title}" }
            h3 { "{value}" }
        }
    }
}

pub(super) fn format_timestamp(timestamp: DateTime<Utc>) -> String {
    timestamp.format("%Y-%m-%d %H:%M UTC").to_string()
}

pub(super) fn format_storage_bytes(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub(super) fn format_storage_bytes_u64(bytes: u64) -> String {
    if bytes > i64::MAX as u64 {
        format!("{} B", bytes)
    } else {
        format_storage_bytes(bytes as i64)
    }
}

pub(super) fn backup_kind_label(semantics: &BackupSemantics) -> &'static str {
    semantics.kind_label()
}

pub(super) fn backup_supports_restore(semantics: &BackupSemantics) -> bool {
    semantics.supports_restore()
}

pub(super) fn restore_database_kind_label(kind: &str) -> &'static str {
    BackupDatabaseFamily::parse(kind).label()
}

pub(super) fn format_storage_mb(storage_used_mb: Option<f64>) -> String {
    storage_used_mb
        .map(|value| format!("{value:.2} MB"))
        .unwrap_or_else(|| "未知".to_string())
}

pub(super) fn summary_value(value: String) -> String {
    let value = value.trim().to_string();
    if value.is_empty() {
        "未配置".to_string()
    } else {
        value
    }
}

pub(super) fn short_identifier(value: &str) -> String {
    value.chars().take(8).collect()
}

pub(super) fn optional_short_id(value: Option<&str>) -> String {
    value
        .map(short_identifier)
        .unwrap_or_else(|| "未知".to_string())
}

pub(super) fn format_json_details(details: &serde_json::Value) -> String {
    serde_json::to_string_pretty(details)
        .or_else(|_| serde_json::to_string(details))
        .unwrap_or_else(|_| "无法序列化详情".to_string())
}

pub(super) fn textarea_rows(value: &str) -> usize {
    let lines = value.lines().count().max(1);
    lines.clamp(3, 8)
}

pub(super) fn page_window(page: i32, page_size: i32, total: i64) -> (i64, i64) {
    if total <= 0 {
        return (0, 0);
    }

    let page = page.max(1) as i64;
    let page_size = page_size.max(1) as i64;
    let start = (page - 1) * page_size + 1;
    let end = (page * page_size).min(total);
    (start, end)
}
