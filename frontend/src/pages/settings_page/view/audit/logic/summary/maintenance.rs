use crate::types::api::AuditLog;

use super::super::super::super::shared::{format_storage_bytes, restore_database_kind_label};
use super::super::super::details::{
    audit_detail_i64, audit_detail_str, audit_detail_string_list, audit_error_summary,
};

pub(super) fn audit_maintenance_summary(log: &AuditLog) -> Option<String> {
    match log.action.as_str() {
        "admin.maintenance.expired_images_cleanup.completed" | "expire_completed" => Some(
            if let Some(count) = audit_detail_i64(log.details.as_ref(), "affected_count") {
                format!("共永久删除 {} 张已过期图片。", count)
            } else {
                "已永久删除所有符合条件的过期图片。".to_string()
            },
        ),
        "admin.maintenance.expired_images_cleanup.failed" | "expire_failed" => {
            Some(audit_error_summary(log).unwrap_or_else(|| {
                "过期图片永久删除失败，请检查图片数据与数据库状态。".to_string()
            }))
        }
        "admin.maintenance.database_backup.created" | "backup_created" => Some(
            audit_detail_str(log.details.as_ref(), "filename")
                .map(|filename| format!("备份文件已生成：{}。", filename))
                .unwrap_or_else(|| "数据库备份已生成。".to_string()),
        ),
        "admin.maintenance.database_backup.downloaded" => {
            let filename = audit_detail_str(log.details.as_ref(), "filename")
                .unwrap_or_else(|| "数据库备份".to_string());
            Some(match audit_detail_i64(log.details.as_ref(), "size_bytes") {
                Some(size_bytes) => format!(
                    "已下载备份文件 {}，大小 {}。",
                    filename,
                    format_storage_bytes(size_bytes)
                ),
                None => format!("已下载备份文件 {}。", filename),
            })
        }
        "admin.maintenance.database_backup.deleted" => {
            let filename = audit_detail_str(log.details.as_ref(), "filename")
                .unwrap_or_else(|| "数据库备份".to_string());
            Some(match audit_detail_i64(log.details.as_ref(), "size_bytes") {
                Some(size_bytes) => format!(
                    "已删除备份文件 {}，释放 {}。",
                    filename,
                    format_storage_bytes(size_bytes)
                ),
                None => format!("已删除备份文件 {}。", filename),
            })
        }
        "admin.maintenance.database_backup.delete_failed" => {
            Some(audit_error_summary(log).unwrap_or_else(|| {
                "删除数据库备份失败，请检查文件是否仍存在以及备份目录权限。".to_string()
            }))
        }
        "admin.maintenance.database_backup.failed" => {
            Some(audit_error_summary(log).unwrap_or_else(|| {
                "数据库备份失败，请检查备份目录、数据库连接和导出命令。".to_string()
            }))
        }
        "admin.maintenance.database_restore.prechecked" => Some(restore_prechecked_summary(log)),
        "admin.maintenance.database_restore.precheck_failed" => {
            Some(restore_precheck_failed_summary(log))
        }
        "admin.maintenance.database_restore.scheduled" => Some(restore_scheduled_summary(log)),
        "system.database_restore.completed" => Some(restore_completed_summary(log)),
        "system.database_restore.rollback_applied" => Some(
            audit_detail_str(log.details.as_ref(), "message").unwrap_or_else(|| {
                "恢复后的数据库启动失败，系统已自动回滚到恢复前快照。".to_string()
            }),
        ),
        "system.database_restore.failed" => Some(
            audit_detail_str(log.details.as_ref(), "message").unwrap_or_else(|| {
                "数据库恢复失败，请检查最后一次恢复结果与启动日志。".to_string()
            }),
        ),
        _ => None,
    }
}

fn restore_prechecked_summary(log: &AuditLog) -> String {
    let database_kind = audit_detail_str(log.details.as_ref(), "backup_database_kind")
        .unwrap_or_else(|| "sqlite".to_string());
    let database_label = restore_database_kind_label(&database_kind);
    let filename = audit_detail_str(log.details.as_ref(), "filename")
        .unwrap_or_else(|| format!("{database_label} 备份"));
    let warnings = audit_detail_string_list(log.details.as_ref(), "warnings");
    if warnings.is_empty() {
        format!(
            "备份 {} 已通过恢复预检，可以在下一次重启前执行恢复。",
            filename
        )
    } else {
        format!(
            "备份 {} 已通过恢复预检；注意事项：{}。",
            filename,
            warnings.join("；")
        )
    }
}

fn restore_precheck_failed_summary(log: &AuditLog) -> String {
    let database_kind = audit_detail_str(log.details.as_ref(), "backup_database_kind")
        .unwrap_or_else(|| "sqlite".to_string());
    let database_label = restore_database_kind_label(&database_kind);
    let filename = audit_detail_str(log.details.as_ref(), "filename")
        .unwrap_or_else(|| format!("{database_label} 备份"));
    let blockers = audit_detail_string_list(log.details.as_ref(), "blockers");
    if blockers.is_empty() {
        format!("备份 {} 未通过恢复预检。", filename)
    } else {
        format!(
            "备份 {} 未通过恢复预检：{}。",
            filename,
            blockers.join("；")
        )
    }
}

fn restore_scheduled_summary(log: &AuditLog) -> String {
    let database_kind = audit_detail_str(log.details.as_ref(), "database_kind")
        .unwrap_or_else(|| "sqlite".to_string());
    let database_label = restore_database_kind_label(&database_kind);
    let filename = audit_detail_str(log.details.as_ref(), "filename")
        .unwrap_or_else(|| format!("{database_label} 备份"));
    format!(
        "{} 的恢复计划已写入；需要尽快重启服务，真正恢复会在下一次启动前执行。",
        filename
    )
}

fn restore_completed_summary(log: &AuditLog) -> String {
    let database_kind = audit_detail_str(log.details.as_ref(), "database_kind")
        .unwrap_or_else(|| "sqlite".to_string());
    let database_label = restore_database_kind_label(&database_kind);
    let filename = audit_detail_str(log.details.as_ref(), "filename")
        .unwrap_or_else(|| format!("{database_label} 备份"));
    let rollback_filename = audit_detail_str(log.details.as_ref(), "rollback_filename");
    match rollback_filename {
        Some(rollback) => format!(
            "备份 {} 已完成恢复，同时保留回滚快照 {} 以便必要时手工追溯。",
            filename, rollback
        ),
        None => format!("备份 {} 已在启动前完成恢复。", filename),
    }
}
