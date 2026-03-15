use crate::components::ConfirmationTone;

use super::models::ConfirmationPlan;

pub(crate) fn advanced_setting_confirmation_plan(key: &str, next_value: &str) -> ConfirmationPlan {
    let next_value_preview = if next_value.trim().is_empty() {
        "空值".to_string()
    } else {
        format!("新值将更新为：{}", truncate_for_confirmation(next_value))
    };

    if setting_is_high_risk(key) {
        ConfirmationPlan {
            title: format!("确认修改 {}", key),
            summary: format!(
                "{}。这类底层存储配置会在保存后立即切换，错误配置会直接影响运行时读写。",
                advanced_setting_confirm_message(key)
            ),
            consequences: vec![
                next_value_preview,
                "错误配置会影响上传、读取或对象存储访问。".to_string(),
                "保存后建议立刻检查健康状态并做一次上传/读取验证。".to_string(),
            ],
            confirm_label: "确认保存".to_string(),
            cancel_label: "取消".to_string(),
            tone: ConfirmationTone::Danger,
            confirm_phrase: Some(key.to_string()),
            confirm_hint: Some(format!("请输入 {} 以确认修改", key)),
        }
    } else {
        ConfirmationPlan {
            title: format!("确认更新 {}", key),
            summary: advanced_setting_confirm_message(key).to_string(),
            consequences: vec![
                next_value_preview,
                "这会直接写入底层 settings 表。".to_string(),
            ],
            confirm_label: "继续保存".to_string(),
            cancel_label: "取消".to_string(),
            tone: ConfirmationTone::Warning,
            confirm_phrase: None,
            confirm_hint: None,
        }
    }
}

pub(crate) fn setting_is_high_risk(key: &str) -> bool {
    matches!(
        key,
        "storage_backend"
            | "local_storage_path"
            | "s3_endpoint"
            | "s3_region"
            | "s3_bucket"
            | "s3_prefix"
            | "s3_force_path_style"
    )
}

fn advanced_setting_confirm_message(key: &str) -> &'static str {
    match key {
        "storage_backend" => "确认修改 storage_backend 吗？保存后后端会立即切换存储方式。",
        "local_storage_path" => "确认修改 local_storage_path 吗？保存后新的写入目录会立即生效。",
        "s3_endpoint" | "s3_region" | "s3_bucket" | "s3_prefix" => {
            "确认修改这项 S3 设置吗？保存后会立即使用新配置，错误配置会影响对象存储读写。"
        }
        "s3_force_path_style" => {
            "确认修改 s3_force_path_style 吗？保存后会立即影响 S3/MinIO 的访问方式。"
        }
        _ => "确认保存这项高级设置吗？",
    }
}

fn truncate_for_confirmation(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() > 60 {
        format!("{}...", trimmed.chars().take(60).collect::<String>())
    } else {
        trimmed.to_string()
    }
}
