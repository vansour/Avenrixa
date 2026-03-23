use crate::types::api::BackupFileSummary;
use dioxus::prelude::*;

use super::super::super::shared::{backup_kind_label, format_storage_bytes_u64, format_timestamp};

pub(super) fn render_backup_files_section(
    backup_files: Vec<(BackupFileSummary, String)>,
    deleting_backup_filename: Option<String>,
    is_loading_backups: bool,
    maintenance_busy: bool,
    on_refresh_backups: EventHandler<MouseEvent>,
    on_delete_backup: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "settings-subcard",
            h3 { "备份文件" }
            p { class: "settings-action-note", "页面恢复入口已移除；如需恢复，请先下载备份，再使用运维脚本执行恢复。" }

            div { class: "settings-list-toolbar",
                div { class: "settings-toolbar-meta",
                    span { class: "stat-pill", "可下载备份 {backup_files.len()} 个" }
                    if is_loading_backups {
                        span { class: "stat-pill stat-pill-warning", "列表刷新中" }
                    }
                }
                div { class: "settings-inline-actions",
                    button {
                        class: "btn",
                        disabled: is_loading_backups || maintenance_busy,
                        onclick: move |event| on_refresh_backups.call(event),
                        if is_loading_backups { "刷新中..." } else { "刷新备份列表" }
                    }
                }
            }

            if is_loading_backups && backup_files.is_empty() {
                div { class: "settings-placeholder settings-placeholder-compact",
                    h3 { "正在加载备份列表" }
                }
            } else if backup_files.is_empty() {
                div { class: "settings-placeholder settings-placeholder-compact",
                    h3 { "暂时没有可下载的备份" }
                }
            } else {
                div { class: "settings-entity-list",
                    {backup_files.into_iter().map(|(backup, download_url)| {
                        let filename_for_download = backup.filename.clone();
                        let filename_for_delete = backup.filename.clone();
                        let kind_label = backup_kind_label(&backup.semantics);
                        let backup_meta = format!(
                            "{} · {}",
                            format_timestamp(backup.created_at),
                            format_storage_bytes_u64(backup.size_bytes)
                        );
                        let is_row_deleting = deleting_backup_filename
                            .as_deref()
                            .is_some_and(|value| value == backup.filename.as_str());
                        rsx! {
                            article { class: "settings-entity-card",
                                div { class: "settings-entity-main",
                                    div { class: "settings-entity-copy",
                                        div { class: "settings-entity-title",
                                            h3 { "{backup.filename}" }
                                            div { class: "settings-kv-badges",
                                                span { class: "settings-kv-badge", "{kind_label}" }
                                            }
                                        }
                                        p { class: "settings-entity-meta", "{backup_meta}" }
                                        p { class: "settings-action-note", "逻辑备份仅供下载；恢复统一走运维脚本。" }
                                    }

                                    div { class: "settings-entity-controls",
                                        a {
                                            class: "btn btn-primary",
                                            href: "{download_url}",
                                            download: "{filename_for_download}",
                                            "下载备份"
                                        }
                                        button {
                                            class: "btn btn-danger",
                                            disabled: maintenance_busy || is_loading_backups,
                                            onclick: move |_| on_delete_backup.call(filename_for_delete.clone()),
                                            if is_row_deleting { "删除中..." } else { "删除备份" }
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
