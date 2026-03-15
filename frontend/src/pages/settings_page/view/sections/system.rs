use crate::types::api::{HealthStatus, SystemStats};
use dioxus::prelude::*;

use super::super::shared::{
    format_storage_bytes, format_storage_mb, render_component_status_card, render_metric_card,
};

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
                    article { class: format!("settings-summary-card {}", health.status.surface_class()),
                        p { class: "settings-summary-label", "系统状态" }
                        h3 { "{health.status.label()}" }
                    }
                    article { class: "settings-summary-card",
                        p { class: "settings-summary-label", "运行版本" }
                        h3 { {health.version.unwrap_or_else(|| "未提供".to_string())} }
                    }
                }

                div { class: "settings-status-grid",
                    {render_component_status_card("数据库", &health.database)}
                    {render_component_status_card("缓存服务", &health.cache)}
                    {render_component_status_card("存储后端", &health.storage)}
                }

                if let Some(metrics) = health.metrics {
                    div { class: "settings-metric-grid",
                        {render_metric_card("健康检查图片数", metrics.images_count.to_string())}
                        {render_metric_card("健康检查用户数", metrics.users_count.to_string())}
                        {render_metric_card("估算存储用量", format_storage_mb(metrics.storage_used_mb))}
                    }
                }
            } else if is_loading {
                div { class: "settings-placeholder settings-placeholder-compact",
                    h3 { "正在加载系统状态" }
                }
            }

            if let Some(stats) = stats {
                div { class: "settings-metric-grid",
                    {render_metric_card("总用户数", stats.total_users.to_string())}
                    {render_metric_card("活跃图片数", stats.total_images.to_string())}
                    {render_metric_card("存储占用", format_storage_bytes(stats.total_storage))}
                    {render_metric_card("累计浏览量", stats.total_views.to_string())}
                    {render_metric_card("近 24 小时新增", stats.images_last_24h.to_string())}
                    {render_metric_card("近 7 天新增", stats.images_last_7d.to_string())}
                }
            }
        }
    }
}
