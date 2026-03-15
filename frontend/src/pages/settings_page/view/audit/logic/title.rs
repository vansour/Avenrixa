use crate::types::api::AuditLog;

pub(super) fn humanize_action(action: &str) -> String {
    action
        .split(['_', '.'])
        .filter(|segment| !segment.trim().is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn audit_title(log: &AuditLog) -> String {
    match log.action.as_str() {
        "admin.maintenance.expired_images_cleanup.completed" | "expire_completed" => {
            "已永久删除过期图片".to_string()
        }
        "admin.maintenance.expired_images_cleanup.failed" | "expire_failed" => {
            "永久删除过期图片失败".to_string()
        }
        "admin.maintenance.database_backup.created" | "backup_created" => {
            "已创建数据库备份".to_string()
        }
        "admin.maintenance.database_backup.downloaded" => "已下载数据库备份".to_string(),
        "admin.maintenance.database_backup.deleted" => "已删除数据库备份".to_string(),
        "admin.maintenance.database_backup.delete_failed" => "删除数据库备份失败".to_string(),
        "admin.maintenance.database_backup.failed" => "数据库备份失败".to_string(),
        "admin.maintenance.database_restore.prechecked" => "数据库恢复预检已通过".to_string(),
        "admin.maintenance.database_restore.precheck_failed" => "数据库恢复预检未通过".to_string(),
        "admin.maintenance.database_restore.scheduled" => "已写入数据库恢复计划".to_string(),
        "system.database_restore.completed" => "数据库恢复已完成".to_string(),
        "system.database_restore.rollback_applied" => "数据库恢复后已自动回滚".to_string(),
        "system.database_restore.failed" => "数据库恢复失败".to_string(),
        "admin.user.role_updated" => "已更新用户角色".to_string(),
        "admin.settings.config_updated" => "已保存系统设置".to_string(),
        "admin.settings.raw_setting_updated" => "已更新原始设置项".to_string(),
        "system.install_completed" => "已完成安装向导".to_string(),
        "image.upload" => "已上传图片".to_string(),
        "image.view" => "已访问图片".to_string(),
        "user.login" => "用户已登录".to_string(),
        _ => humanize_action(&log.action),
    }
}

pub(super) fn audit_target_type_label(target_type: &str) -> String {
    match target_type {
        "maintenance" => "维护任务".to_string(),
        "settings" => "系统设置".to_string(),
        "setting" => "原始设置项".to_string(),
        "user" => "用户".to_string(),
        "image" => "图片".to_string(),
        "system" => "系统".to_string(),
        _ => humanize_action(target_type),
    }
}
