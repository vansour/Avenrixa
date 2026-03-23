mod backups;

use crate::types::api::{BackupFileSummary, BackupResponse};
use dioxus::prelude::*;

use self::backups::render_backup_files_section;
use super::super::shared::{format_timestamp, render_metric_card};

#[component]
pub fn MaintenanceSettingsSection(
    error_message: String,
    success_message: String,
    last_backup: Option<BackupResponse>,
    backup_files: Vec<(BackupFileSummary, String)>,
    last_expired_cleanup_count: Option<i64>,
    is_cleaning_expired: bool,
    is_backing_up: bool,
    deleting_backup_filename: Option<String>,
    is_loading_backups: bool,
    #[props(default)] on_cleanup_expired: EventHandler<MouseEvent>,
    #[props(default)] on_backup: EventHandler<MouseEvent>,
    #[props(default)] on_refresh_backups: EventHandler<MouseEvent>,
    #[props(default)] on_delete_backup: EventHandler<String>,
) -> Element {
    let last_backup_name = last_backup
        .as_ref()
        .map(|backup| backup.filename.clone())
        .unwrap_or_else(|| "暂无备份".to_string());
    let last_backup_time = last_backup
        .as_ref()
        .map(|backup| format_timestamp(backup.created_at))
        .unwrap_or_else(|| "未生成".to_string());
    let expired_cleanup_summary = last_expired_cleanup_count
        .map(|count| format!("{} 张图片", count))
        .unwrap_or_else(|| "未执行".to_string());
    let maintenance_busy =
        is_cleaning_expired || is_backing_up || deleting_backup_filename.is_some();

    rsx! {
        div { class: "settings-stack",
            if !error_message.is_empty() {
                div { class: "error-banner", "{error_message}" }
            }

            if !success_message.is_empty() {
                div { class: "settings-banner settings-banner-success", "{success_message}" }
            }

            div { class: "settings-metric-grid",
                {render_metric_card("最近备份文件", last_backup_name)}
                {render_metric_card("最近备份时间", last_backup_time)}
                {render_metric_card("最近过期删除", expired_cleanup_summary)}
            }

            div { class: "settings-action-grid",
                article { class: "settings-action-card",
                    div { class: "settings-action-copy",
                        div { class: "settings-action-meta",
                            span { class: "settings-risk-badge is-danger", "Danger" }
                        }
                        h3 { "永久删除过期图片" }
                    }
                    button {
                        class: "btn btn-danger",
                        disabled: maintenance_busy,
                        onclick: move |event| on_cleanup_expired.call(event),
                        if is_cleaning_expired { "删除中..." } else { "执行删除" }
                    }
                }

                article { class: "settings-action-card settings-action-card-accent",
                    div { class: "settings-action-copy",
                        div { class: "settings-action-meta",
                            span { class: "settings-risk-badge is-safe", "Safe" }
                        }
                        h3 { "数据库备份" }
                    }
                    button {
                        class: "btn btn-primary",
                        disabled: maintenance_busy,
                        onclick: move |event| on_backup.call(event),
                        if is_backing_up { "备份中..." } else { "生成备份" }
                    }
                }
            }

            {render_backup_files_section(
                backup_files,
                deleting_backup_filename,
                is_loading_backups,
                maintenance_busy,
                on_refresh_backups,
                on_delete_backup,
            )}
        }
    }
}
