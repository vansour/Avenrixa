use crate::types::api::AuditLog;

use super::super::super::shared::optional_short_id;
use super::super::details::audit_detail_str;

pub(super) fn audit_category_label(log: &AuditLog) -> &'static str {
    match log.action.as_str() {
        "admin.maintenance.expired_images_cleanup.completed"
        | "admin.maintenance.expired_images_cleanup.failed"
        | "expire_completed"
        | "expire_failed"
        | "admin.maintenance.database_backup.created"
        | "admin.maintenance.database_backup.deleted"
        | "admin.maintenance.database_backup.delete_failed"
        | "admin.maintenance.database_backup.failed"
        | "admin.maintenance.database_backup.downloaded"
        | "admin.maintenance.database_restore.prechecked"
        | "admin.maintenance.database_restore.precheck_failed"
        | "admin.maintenance.database_restore.scheduled"
        | "system.database_restore.completed"
        | "system.database_restore.rollback_applied"
        | "system.database_restore.failed"
        | "backup_created" => "维护操作",
        "admin.user.role_updated" => "权限变更",
        "admin.settings.config_updated" | "admin.settings.raw_setting_updated" => "设置变更",
        "system.install_completed" => "安装初始化",
        "image.upload" | "image.view" => "图片操作",
        "user.login" => "认证事件",
        _ => "系统事件",
    }
}

pub(super) fn audit_category_class(log: &AuditLog) -> &'static str {
    match audit_risk_value(log).as_deref() {
        Some("danger") => "is-danger",
        Some("warning") => "is-warning",
        Some("info") => "is-info",
        _ => "is-neutral",
    }
}

pub(super) fn audit_risk_label(log: &AuditLog) -> &'static str {
    match audit_risk_value(log).as_deref() {
        Some("danger") => "高风险",
        Some("warning") => "需确认",
        Some("info") => "常规",
        _ => "事件",
    }
}

fn audit_risk_value(log: &AuditLog) -> Option<String> {
    audit_detail_str(log.details.as_ref(), "risk_level").or_else(|| match log.action.as_str() {
        "admin.maintenance.database_backup.deleted"
        | "admin.maintenance.database_backup.delete_failed"
        | "admin.maintenance.database_restore.prechecked"
        | "admin.maintenance.database_restore.precheck_failed"
        | "admin.maintenance.database_restore.scheduled"
        | "system.database_restore.completed"
        | "system.database_restore.rollback_applied"
        | "system.database_restore.failed"
        | "admin.user.role_updated" => Some("danger".to_string()),
        "admin.maintenance.expired_images_cleanup.completed"
        | "admin.maintenance.expired_images_cleanup.failed"
        | "expire_completed"
        | "expire_failed"
        | "admin.settings.config_updated"
        | "admin.settings.raw_setting_updated" => Some("warning".to_string()),
        "admin.maintenance.database_backup.created"
        | "admin.maintenance.database_backup.failed"
        | "admin.maintenance.database_backup.downloaded"
        | "backup_created"
        | "system.install_completed"
        | "image.upload"
        | "image.view"
        | "user.login" => Some("info".to_string()),
        _ => None,
    })
}

fn is_bootstrap_admin_id(value: &str) -> bool {
    value == "00000000-0000-0000-0000-000000000001"
}

fn is_maintenance_action(action: &str, target_type: &str) -> bool {
    matches!(
        action,
        "admin.maintenance.expired_images_cleanup.completed"
            | "admin.maintenance.expired_images_cleanup.failed"
            | "expire_completed"
            | "expire_failed"
            | "admin.maintenance.database_backup.created"
            | "admin.maintenance.database_backup.deleted"
            | "admin.maintenance.database_backup.delete_failed"
            | "admin.maintenance.database_backup.failed"
            | "admin.maintenance.database_backup.downloaded"
            | "admin.maintenance.database_restore.prechecked"
            | "admin.maintenance.database_restore.precheck_failed"
            | "admin.maintenance.database_restore.scheduled"
            | "system.database_restore.completed"
            | "system.database_restore.rollback_applied"
            | "system.database_restore.failed"
            | "backup_created"
    ) || target_type == "maintenance"
}

pub(super) fn audit_actor_label(log: &AuditLog) -> String {
    audit_detail_str(log.details.as_ref(), "admin_email")
        .or_else(|| audit_detail_str(log.details.as_ref(), "actor_email"))
        .or_else(|| audit_detail_str(log.details.as_ref(), "email"))
        .or_else(|| {
            if log.action == "user.login" {
                Some("当前用户".to_string())
            } else if log.action == "system.install_completed" {
                Some("安装向导".to_string())
            } else if is_maintenance_action(&log.action, &log.target_type)
                && log.user_id.as_deref().is_some_and(is_bootstrap_admin_id)
            {
                Some("系统任务".to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| optional_short_id(log.user_id.as_deref()))
}
