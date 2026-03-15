pub(super) fn audit_error_summary(log: &crate::types::api::AuditLog) -> Option<String> {
    audit_detail_str(log.details.as_ref(), "error").map(|error| format!("错误信息：{}。", error))
}

pub(super) fn audit_detail_str(details: Option<&serde_json::Value>, key: &str) -> Option<String> {
    details?.get(key)?.as_str().map(|value| value.to_string())
}

pub(super) fn audit_detail_i64(details: Option<&serde_json::Value>, key: &str) -> Option<i64> {
    details?.get(key)?.as_i64()
}

pub(super) fn audit_detail_bool(details: Option<&serde_json::Value>, key: &str) -> bool {
    details
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

pub(super) fn audit_detail_string_list(
    details: Option<&serde_json::Value>,
    key: &str,
) -> Vec<String> {
    details
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(|value| value.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn setting_key_label(key: &str) -> String {
    match key {
        "site_name" => "网站名称".to_string(),
        "storage_backend" => "存储后端".to_string(),
        "local_storage_path" => "本地存储路径".to_string(),
        "mail_enabled" => "邮件服务开关".to_string(),
        "mail_smtp_host" => "SMTP 主机".to_string(),
        "mail_smtp_port" => "SMTP 端口".to_string(),
        "mail_smtp_user" => "SMTP 用户名".to_string(),
        "mail_smtp_password" => "SMTP 密码".to_string(),
        "mail_from_email" => "发件邮箱".to_string(),
        "mail_from_name" => "发件人名称".to_string(),
        "mail_link_base_url" => "站点访问地址（邮件链接）".to_string(),
        "s3_endpoint" => "对象存储服务地址".to_string(),
        "s3_region" => "对象存储区域".to_string(),
        "s3_bucket" => "对象存储桶".to_string(),
        "s3_prefix" => "对象存储目录前缀".to_string(),
        "s3_access_key" => "对象存储访问密钥".to_string(),
        "s3_secret_key" => "对象存储私有密钥".to_string(),
        "s3_force_path_style" => "对象存储路径风格".to_string(),
        _ => key.to_string(),
    }
}
