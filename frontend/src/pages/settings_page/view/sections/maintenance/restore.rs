use crate::types::api::BackupRestoreStatusResponse;
use dioxus::prelude::*;

use super::super::super::shared::{format_storage_bytes_u64, format_timestamp, render_metric_card};

pub(super) fn render_restore_section(
    restore_status: Option<BackupRestoreStatusResponse>,
    is_loading_restore_status: bool,
    maintenance_busy: bool,
    on_refresh_restore_status: EventHandler<MouseEvent>,
) -> Element {
    let pending_restore = restore_status
        .as_ref()
        .and_then(|status| status.pending.clone());
    let last_restore_result = restore_status
        .as_ref()
        .and_then(|status| status.last_result.clone());
    let pending_restore_summary = pending_restore
        .as_ref()
        .map(|item| item.filename.clone())
        .unwrap_or_else(|| "无".to_string());
    let pending_restore_time = pending_restore
        .as_ref()
        .map(|item| format_timestamp(item.scheduled_at))
        .unwrap_or_else(|| "未计划".to_string());
    let last_restore_status = last_restore_result
        .as_ref()
        .map(|item| item.status.label().to_string())
        .unwrap_or_else(|| "暂无记录".to_string());
    let last_restore_time = last_restore_result
        .as_ref()
        .map(|item| format_timestamp(item.finished_at))
        .unwrap_or_else(|| "未执行".to_string());
    let pending_restore_count = if pending_restore.is_some() { 1 } else { 0 };

    rsx! {
        div { class: "settings-subcard",
            h3 { "数据库恢复状态" }

            div { class: "settings-list-toolbar",
                div { class: "settings-toolbar-meta",
                    span { class: "stat-pill", "待执行计划 {pending_restore_count}" }
                    if is_loading_restore_status {
                        span { class: "stat-pill stat-pill-warning", "状态刷新中" }
                    }
                }
                div { class: "settings-inline-actions",
                    button {
                        class: "btn",
                        disabled: maintenance_busy || is_loading_restore_status,
                        onclick: move |event| on_refresh_restore_status.call(event),
                        if is_loading_restore_status { "刷新中..." } else { "刷新恢复状态" }
                    }
                }
            }

            if is_loading_restore_status && restore_status.is_none() {
                div { class: "settings-placeholder settings-placeholder-compact",
                    h3 { "正在加载恢复状态" }
                }
            } else {
                div { class: "settings-metric-grid",
                    {render_metric_card("待执行恢复", pending_restore_summary)}
                    {render_metric_card("计划写入时间", pending_restore_time)}
                    {render_metric_card("最近恢复结果", last_restore_status)}
                    {render_metric_card("最近结果时间", last_restore_time)}
                }

                if let Some(pending) = pending_restore.clone() {
                    div { class: "settings-banner settings-banner-warning",
                        "检测到待执行恢复计划，请重启服务完成恢复。"
                    }
                    article { class: "settings-entity-card",
                        div { class: "settings-entity-main",
                            div { class: "settings-entity-copy",
                                div { class: "settings-entity-title",
                                    h3 { "{pending.filename}" }
                                    span { class: "settings-kv-badge is-warning", "{pending.database_kind.label()} · 待执行" }
                                }
                                p { class: "settings-entity-meta",
                                    "计划写入于 {format_timestamp(pending.scheduled_at)} · 备份创建于 {format_timestamp(pending.backup_created_at)} · {format_storage_bytes_u64(pending.backup_size_bytes)}"
                                }
                                p { class: "settings-action-note",
                                    "申请人 {pending.requested_by_email} · 完成后当前登录会话会失效"
                                }
                            }
                        }
                    }
                }

                if let Some(result) = last_restore_result {
                    article { class: "settings-entity-card",
                        div { class: "settings-entity-main",
                            div { class: "settings-entity-copy",
                                div { class: "settings-entity-title",
                                    h3 { "最近一次恢复结果" }
                                    span {
                                        class: format!(
                                            "settings-kv-badge {}",
                                            result.status.surface_class()
                                        ),
                                        "{result.status.label()}"
                                    }
                                }
                                p { class: "settings-entity-meta",
                                    "{result.database_kind.label()} 备份 {result.filename} · 完成于 {format_timestamp(result.finished_at)}"
                                }
                                p { class: "settings-action-note", "{result.message}" }
                                if let Some(rollback_filename) = result.rollback_filename {
                                    p { class: "settings-entity-meta", "回滚快照 {rollback_filename}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
