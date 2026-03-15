mod backups;
mod restore;

use crate::types::api::{BackupFileSummary, BackupResponse, BackupRestoreStatusResponse};
use dioxus::prelude::*;

use self::backups::render_backup_files_section;
use self::restore::render_restore_section;
use super::super::shared::{format_timestamp, render_metric_card};

#[component]
pub fn MaintenanceSettingsSection(
    error_message: String,
    success_message: String,
    last_backup: Option<BackupResponse>,
    backup_files: Vec<(BackupFileSummary, String)>,
    restore_status: Option<BackupRestoreStatusResponse>,
    last_expired_cleanup_count: Option<i64>,
    is_cleaning_expired: bool,
    is_backing_up: bool,
    deleting_backup_filename: Option<String>,
    processing_restore_filename: Option<String>,
    is_loading_backups: bool,
    is_loading_restore_status: bool,
    #[props(default)] on_cleanup_expired: EventHandler<MouseEvent>,
    #[props(default)] on_backup: EventHandler<MouseEvent>,
    #[props(default)] on_refresh_backups: EventHandler<MouseEvent>,
    #[props(default)] on_refresh_restore_status: EventHandler<MouseEvent>,
    #[props(default)] on_delete_backup: EventHandler<String>,
    #[props(default)] on_restore_backup: EventHandler<BackupFileSummary>,
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
    let pending_restore_filename = restore_status
        .as_ref()
        .and_then(|status| status.pending.as_ref())
        .map(|item| item.filename.clone());
    let maintenance_busy = is_cleaning_expired
        || is_backing_up
        || deleting_backup_filename.is_some()
        || processing_restore_filename.is_some();
    let has_pending_restore = pending_restore_filename.is_some();

    rsx! {
        div { class: "settings-stack",
            if !error_message.is_empty() {
                div { class: "error-banner", "{error_message}" }
            }

            if !success_message.is_empty() {
                div { class: "settings-banner settings-banner-success", "{success_message}" }
            }

            div { class: "settings-banner settings-banner-neutral",
                "维护工具已启用分级确认：过期图片永久删除和数据库恢复都属于高风险操作，需要输入确认词；数据库备份可直接执行。"
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
                        p { class: "settings-action-note", "批量删除所有已过期图片，并同步移除文件与数据库记录。" }
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
                        p { class: "settings-action-note", "生成当前数据库级备份；SQLite 会导出数据库快照，MySQL / MariaDB 与 PostgreSQL 会导出逻辑备份。" }
                    }
                    button {
                        class: "btn btn-primary",
                        disabled: maintenance_busy,
                        onclick: move |event| on_backup.call(event),
                        if is_backing_up { "备份中..." } else { "生成备份" }
                    }
                }
            }

            {render_restore_section(
                restore_status,
                is_loading_restore_status,
                maintenance_busy,
                on_refresh_restore_status,
            )}

            {render_backup_files_section(
                backup_files,
                pending_restore_filename,
                deleting_backup_filename,
                processing_restore_filename,
                is_loading_backups,
                is_loading_restore_status,
                maintenance_busy,
                has_pending_restore,
                on_refresh_backups,
                on_delete_backup,
                on_restore_backup,
            )}
        }
    }
}
