use crate::types::api::{AuditLog, StorageBackendKind, UserRole};

use super::super::super::details::{
    audit_detail_bool, audit_detail_str, audit_detail_string_list, setting_key_label,
};

pub(super) fn audit_settings_summary(log: &AuditLog) -> Option<String> {
    match log.action.as_str() {
        "admin.user.role_updated" => Some(role_updated_summary(log)),
        "admin.settings.config_updated" => Some(config_updated_summary(log)),
        "admin.settings.raw_setting_updated" => Some(raw_setting_updated_summary(log)),
        "system.install_completed" => Some(install_completed_summary(log)),
        _ => None,
    }
}

fn role_updated_summary(log: &AuditLog) -> String {
    let email = audit_detail_str(log.details.as_ref(), "user_email")
        .unwrap_or_else(|| "目标用户".to_string());
    let previous_role = audit_detail_str(log.details.as_ref(), "previous_role")
        .unwrap_or_else(|| "unknown".to_string());
    let new_role =
        audit_detail_str(log.details.as_ref(), "new_role").unwrap_or_else(|| "unknown".to_string());
    format!(
        "{} 的角色已从 {} 调整为 {}。",
        email,
        UserRole::parse(&previous_role).label(),
        UserRole::parse(&new_role).label()
    )
}

fn config_updated_summary(log: &AuditLog) -> String {
    let changed_keys = audit_detail_string_list(log.details.as_ref(), "changed_keys")
        .into_iter()
        .map(|key| setting_key_label(&key).to_string())
        .collect::<Vec<_>>();
    let restart_required = audit_detail_bool(log.details.as_ref(), "restart_required");
    if changed_keys.is_empty() {
        "已保存结构化系统设置。".to_string()
    } else if restart_required {
        format!(
            "已更新 {}，且这些调整需要重启服务后完全生效。",
            changed_keys.join("、")
        )
    } else {
        format!("已更新 {}。", changed_keys.join("、"))
    }
}

fn raw_setting_updated_summary(log: &AuditLog) -> String {
    let key = audit_detail_str(log.details.as_ref(), "setting_key")
        .map(|value| setting_key_label(&value).to_string())
        .unwrap_or_else(|| "未知键名".to_string());
    let restart_required = audit_detail_bool(log.details.as_ref(), "restart_required");
    if restart_required {
        format!("已通过高级设置修改 {}，需要重启服务后完全生效。", key)
    } else {
        format!("已通过高级设置修改 {}。", key)
    }
}

fn install_completed_summary(log: &AuditLog) -> String {
    let site_name =
        audit_detail_str(log.details.as_ref(), "site_name").unwrap_or_else(|| "站点".to_string());
    let storage_backend = audit_detail_str(log.details.as_ref(), "storage_backend")
        .unwrap_or_else(|| "local".to_string());
    let storage_label = StorageBackendKind::parse(&storage_backend).label();
    let mail_status = if audit_detail_bool(log.details.as_ref(), "mail_enabled") {
        "邮件已启用"
    } else {
        "邮件未启用"
    };
    let favicon_status = if audit_detail_bool(log.details.as_ref(), "favicon_configured") {
        "图标已配置"
    } else {
        "图标未配置"
    };
    format!(
        "安装完成，站点名称为 {}，存储使用 {}，{}，{}。",
        site_name, storage_label, mail_status, favicon_status
    )
}
