use crate::types::api::AuditLog;

use super::super::super::super::shared::{format_storage_bytes, short_identifier};
use super::super::super::details::{audit_detail_i64, audit_detail_str};

pub(super) fn audit_activity_summary(log: &AuditLog) -> Option<String> {
    match log.action.as_str() {
        "image.upload" => Some(image_upload_summary(log)),
        "image.view" => Some(image_view_summary(log)),
        "user.login" => Some(
            audit_detail_str(log.details.as_ref(), "email")
                .map(|email| format!("{email} 已登录控制台。"))
                .unwrap_or_else(|| "当前账户已登录控制台。".to_string()),
        ),
        _ => None,
    }
}

fn image_upload_summary(log: &AuditLog) -> String {
    let filename = audit_detail_str(log.details.as_ref(), "stored_filename")
        .unwrap_or_else(|| "未命名图片".to_string());
    let mut segments = vec![format!("已上传 {}", filename)];
    if let Some(format) = audit_detail_str(log.details.as_ref(), "format") {
        segments.push(format!("格式 {}", format.to_uppercase()));
    }
    if let Some(size_bytes) = audit_detail_i64(log.details.as_ref(), "size_bytes") {
        segments.push(format!("大小 {}", format_storage_bytes(size_bytes)));
    }
    format!("{}。", segments.join("，"))
}

fn image_view_summary(log: &AuditLog) -> String {
    if let Some(target_id) = log.target_id.as_deref() {
        format!(
            "已记录一次图片访问，目标ID 为 {}。",
            short_identifier(target_id)
        )
    } else {
        "已记录一次图片访问。".to_string()
    }
}
