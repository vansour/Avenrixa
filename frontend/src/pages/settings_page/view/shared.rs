use crate::types::api::{BackupSemantics, ComponentStatus};
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
    let message = status
        .message
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    rsx! {
        article { class: format!("settings-status-card settings-status-card-compact {}", status.status.surface_class()),
            p { class: "settings-summary-label", "{title}" }
            div { class: "settings-status-inline",
                h3 { "{status.status.label()}" }
                if let Some(message) = message {
                    p { class: "settings-status-message", "{message}" }
                }
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

pub(super) fn format_optional_timestamp(timestamp: Option<DateTime<Utc>>) -> String {
    timestamp
        .map(format_timestamp)
        .unwrap_or_else(|| "未记录".to_string())
}

pub(super) fn format_count_option(value: Option<i64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "未知".to_string())
}

pub(super) fn format_duration_ms(value: Option<u64>) -> String {
    value
        .map(|value| format!("{value} ms"))
        .unwrap_or_else(|| "未记录".to_string())
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

pub(super) fn format_storage_mb(storage_used_mb: Option<f64>) -> String {
    storage_used_mb
        .map(|value| format!("{value:.2} MB"))
        .unwrap_or_else(|| "未知".to_string())
}

pub(super) fn short_identifier(value: &str) -> String {
    value.chars().take(8).collect()
}
