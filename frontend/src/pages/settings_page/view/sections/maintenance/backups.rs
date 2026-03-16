use crate::types::api::BackupFileSummary;
use dioxus::prelude::*;

use super::super::super::shared::{
    backup_kind_label, backup_supports_restore, format_storage_bytes_u64, format_timestamp,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn render_backup_files_section(
    backup_files: Vec<(BackupFileSummary, String)>,
    pending_restore_filename: Option<String>,
    deleting_backup_filename: Option<String>,
    processing_restore_filename: Option<String>,
    is_loading_backups: bool,
    is_loading_restore_status: bool,
    maintenance_busy: bool,
    has_pending_restore: bool,
    on_refresh_backups: EventHandler<MouseEvent>,
    on_delete_backup: EventHandler<String>,
    on_restore_backup: EventHandler<BackupFileSummary>,
) -> Element {
    rsx! {
        div { class: "settings-subcard",
            h3 { "备份文件" }
            p { class: "settings-section-copy",
                "这里展示后台生成的数据库级备份。SQLite 数据库快照仍可从当前页面写入恢复计划，但这条能力在 1.0 范围内按 Experimental 保留；MySQL / MariaDB 逻辑导出与 PostgreSQL 导出当前仅支持下载或运维侧恢复。"
            }

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
                        let backup_for_restore = backup.clone();
                        let kind_label = backup_kind_label(&backup.semantics);
                        let backup_meta = format!(
                            "{} · {}",
                            format_timestamp(backup.created_at),
                            format_storage_bytes_u64(backup.size_bytes)
                        );
                        let is_row_deleting = deleting_backup_filename
                            .as_deref()
                            .is_some_and(|value| value == backup.filename.as_str());
                        let is_row_restoring = processing_restore_filename
                            .as_deref()
                            .is_some_and(|value| value == backup.filename.as_str());
                        let is_pending_target = pending_restore_filename
                            .as_deref()
                            .is_some_and(|value| value == backup.filename.as_str());
                        let supports_restore = backup_supports_restore(&backup.semantics);
                        let is_experimental_page_restore =
                            backup.semantics.is_sqlite_database_snapshot();
                        rsx! {
                            article { class: "settings-entity-card",
                                div { class: "settings-entity-main",
                                    div { class: "settings-entity-copy",
                                        div { class: "settings-entity-title",
                                            h3 { "{backup.filename}" }
                                            div { class: "settings-kv-badges",
                                                span { class: "settings-kv-badge", "{kind_label}" }
                                                if is_experimental_page_restore {
                                                    span { class: "settings-kv-badge is-warning", "Experimental" }
                                                }
                                            }
                                        }
                                        p { class: "settings-entity-meta", "{backup_meta}" }
                                        if is_experimental_page_restore {
                                            p { class: "settings-action-note",
                                                "当前页面内的 SQLite 恢复在 1.0 范围内按 Experimental 保留，适合受控环境验证，不属于默认 GA 发布承诺。"
                                            }
                                        } else if !supports_restore {
                                            p { class: "settings-action-note", "当前这类备份仅支持下载或运维侧恢复，不支持当前页面恢复。" }
                                        }
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
                                            disabled: !supports_restore || maintenance_busy || is_loading_restore_status || has_pending_restore,
                                            onclick: move |_| on_restore_backup.call(backup_for_restore.clone()),
                                            if is_row_restoring {
                                                "处理中..."
                                            } else if is_pending_target {
                                                "已计划恢复"
                                            } else if !supports_restore {
                                                "不支持页面恢复"
                                            } else {
                                                "恢复到此备份"
                                            }
                                        }
                                        button {
                                            class: "btn btn-danger",
                                            disabled: maintenance_busy || is_loading_backups || has_pending_restore,
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
