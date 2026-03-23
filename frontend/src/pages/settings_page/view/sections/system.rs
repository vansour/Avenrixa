use crate::types::api::{
    BackgroundTaskMetrics, HealthState, HealthStatus, RuntimeOperationMetrics, SystemStats,
};
use dioxus::prelude::*;

use super::super::shared::{
    format_count_option, format_duration_ms, format_optional_timestamp, format_storage_bytes,
    format_storage_mb, render_component_status_card, render_metric_card,
};

fn operation_is_currently_degraded(metrics: &RuntimeOperationMetrics) -> bool {
    match (
        metrics.last_failure_at.as_ref(),
        metrics.last_success_at.as_ref(),
    ) {
        (Some(last_failure), Some(last_success)) => last_failure >= last_success,
        (Some(_), None) => true,
        (None, _) => false,
    }
}

fn render_runtime_operation_card(title: &str, metrics: &RuntimeOperationMetrics) -> Element {
    let state = if operation_is_currently_degraded(metrics) {
        HealthState::Degraded
    } else {
        HealthState::Healthy
    };

    rsx! {
        article { class: format!("settings-status-card settings-status-card-compact {}", state.surface_class()),
            p { class: "settings-summary-label", "{title}" }
            div { class: "settings-status-inline",
                h3 { "{state.label()}" }
                p { class: "settings-status-message",
                    "成功 {metrics.total_successes} 次 / 失败 {metrics.total_failures} 次"
                }
                p { class: "settings-status-message",
                    "最近耗时 {format_duration_ms(metrics.last_duration_ms)} · 均值 {format_duration_ms(metrics.average_duration_ms)}"
                }
                p { class: "settings-status-message",
                    "最近成功 {format_optional_timestamp(metrics.last_success_at)}"
                }
                if metrics.last_failure_at.is_some() {
                    p { class: "settings-status-message",
                        "最近失败 {format_optional_timestamp(metrics.last_failure_at)}"
                    }
                }
                if let Some(last_error) = metrics.last_error.as_deref() {
                    p { class: "settings-status-message", "{last_error}" }
                }
            }
        }
    }
}

fn render_background_task_card(task: &BackgroundTaskMetrics) -> Element {
    let state = if task.consecutive_failures > 0 {
        HealthState::Degraded
    } else {
        HealthState::Healthy
    };

    rsx! {
        article { class: format!("settings-status-card settings-status-card-compact {}", state.surface_class()),
            p { class: "settings-summary-label", "{task.task_name}" }
            div { class: "settings-status-inline",
                h3 { "{state.label()}" }
                p { class: "settings-status-message",
                    "运行 {task.total_runs} 次 / 失败 {task.total_failures} 次"
                }
                p { class: "settings-status-message",
                    "连续失败 {task.consecutive_failures} 次 · 最近耗时 {format_duration_ms(task.last_duration_ms)}"
                }
                p { class: "settings-status-message",
                    "最近成功 {format_optional_timestamp(task.last_success_at)}"
                }
                if task.total_failures > 0 {
                    p { class: "settings-status-message",
                        "最近失败 {format_optional_timestamp(task.last_failure_at)}"
                    }
                }
                if let Some(last_error) = task.last_error.as_deref() {
                    p { class: "settings-status-message", "{last_error}" }
                }
            }
        }
    }
}

#[component]
pub fn SystemStatusSection(
    health: Option<HealthStatus>,
    stats: Option<SystemStats>,
    error_message: String,
    is_loading: bool,
    #[props(default)] on_refresh: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "settings-stack",
            div { class: "settings-inline-actions",
                button {
                    class: "btn",
                    disabled: is_loading,
                    onclick: move |event| on_refresh.call(event),
                    if is_loading { "刷新中..." } else { "刷新状态" }
                }
            }

            if !error_message.is_empty() {
                div { class: "error-banner", "{error_message}" }
            }

            if let Some(health) = health.clone() {
                div { class: "settings-status-summary",
                    article { class: format!("settings-summary-card settings-summary-card-inline {}", health.status.surface_class()),
                        p { class: "settings-summary-label", "状态" }
                        h3 { class: "settings-summary-value", "{health.status.label()}" }
                    }
                    article { class: "settings-summary-card settings-summary-card-inline",
                        p { class: "settings-summary-label", "版本" }
                        h3 { class: "settings-summary-value", {health.version.unwrap_or_else(|| "未提供".to_string())} }
                    }
                }

                div { class: "settings-status-grid",
                    {render_component_status_card("数据库", &health.database)}
                    {render_component_status_card("缓存服务", &health.cache)}
                    {render_component_status_card("存储后端", &health.storage)}
                    {render_component_status_card("运行态指标", &health.observability)}
                }

                if let Some(metrics) = health.metrics.clone() {
                    div { class: "settings-metric-grid",
                        {render_metric_card("图片数", format_count_option(metrics.images_count))}
                        {render_metric_card("用户数", format_count_option(metrics.users_count))}
                        {render_metric_card("估算用量", format_storage_mb(metrics.storage_used_mb))}
                        {render_metric_card(
                            "运行时长",
                            health
                                .uptime_seconds
                                .map(|seconds| format!("{seconds} s"))
                                .unwrap_or_else(|| "未提供".to_string()),
                        )}
                    }
                }
            } else if is_loading {
                div { class: "settings-placeholder settings-placeholder-compact",
                    h3 { "正在加载系统状态" }
                }
            }

            if let Some(stats) = stats.clone() {
                div { class: "settings-metric-grid",
                    {render_metric_card("总用户数", stats.total_users.to_string())}
                    {render_metric_card("活跃图片数", stats.total_images.to_string())}
                    {render_metric_card("存储占用", format_storage_bytes(stats.total_storage))}
                    {render_metric_card("累计浏览量", stats.total_views.to_string())}
                    {render_metric_card("近 24 小时新增", stats.images_last_24h.to_string())}
                    {render_metric_card("近 7 天新增", stats.images_last_7d.to_string())}
                }

                div { class: "settings-metric-grid",
                    {render_metric_card("清理积压", stats.runtime.backlog.storage_cleanup_pending.to_string())}
                    {render_metric_card("重试队列", stats.runtime.backlog.storage_cleanup_retrying.to_string())}
                    {render_metric_card("审计写入失败", stats.runtime.audit_writes.total_failures.to_string())}
                    {render_metric_card("Refresh 失败", stats.runtime.auth_refresh.total_failures.to_string())}
                    {render_metric_card("图片处理均值", format_duration_ms(stats.runtime.image_processing.average_duration_ms))}
                    {render_metric_card("备份最近耗时", format_duration_ms(stats.runtime.backups.last_duration_ms))}
                }

                div { class: "settings-status-grid",
                    {render_runtime_operation_card("审计写入", &stats.runtime.audit_writes)}
                    {render_runtime_operation_card("认证 Refresh", &stats.runtime.auth_refresh)}
                    {render_runtime_operation_card("图片处理", &stats.runtime.image_processing)}
                    {render_runtime_operation_card("备份任务", &stats.runtime.backups)}
                }

                if !stats.runtime.background_tasks.is_empty() {
                    div { class: "settings-status-grid",
                        for task in &stats.runtime.background_tasks {
                            {render_background_task_card(task)}
                        }
                    }
                }
            }
        }
    }
}
