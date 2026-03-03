use std::collections::HashMap;
use uuid::Uuid;

/// 验证结果
#[allow(dead_code)]
#[derive(Debug)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: HashMap<String, String>,
}

impl ValidationResult {
    #[allow(dead_code)]
    pub fn success() -> Self {
        Self {
            valid: true,
            errors: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn error(field: impl Into<String>, message: impl Into<String>) -> Self {
        let mut errors = HashMap::new();
        errors.insert(field.into(), message.into());
        Self {
            valid: false,
            errors,
        }
    }
}

/// 用户名验证
#[allow(dead_code)]
pub fn validate_username(username: &str) -> ValidationResult {
    if username.len() < 3 {
        return ValidationResult::error("username", "用户名长度至少为 3 个字符");
    }
    if username.len() > 50 {
        return ValidationResult::error("username", "用户名长度不能超过 50 个字符");
    }
    // 检查用户名只包含允许的字符
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return ValidationResult::error("username", "用户名只能包含字母、数字、下划线和连字符");
    }
    ValidationResult::success()
}

/// 密码验证
#[allow(dead_code)]
pub fn validate_password(password: &str) -> ValidationResult {
    if password.len() < 6 {
        return ValidationResult::error("password", "密码长度至少为 6 个字符");
    }
    if password.len() > 128 {
        return ValidationResult::error("password", "密码长度不能超过 128 个字符");
    }
    ValidationResult::success()
}

/// 文件名验证
#[allow(dead_code)]
pub fn validate_filename(filename: &str) -> ValidationResult {
    if filename.is_empty() {
        return ValidationResult::error("filename", "文件名不能为空");
    }
    if filename.len() > 255 {
        return ValidationResult::error("filename", "文件名长度不能超过 255 个字符");
    }
    // 检查路径遍历攻击
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return ValidationResult::error("filename", "文件名包含非法字符");
    }
    // 检查保留文件名
    let reserved = ["CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5",
        "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"];
    if reserved.contains(&filename.to_uppercase().as_str()) {
        return ValidationResult::error("filename", "文件名是系统保留名称");
    }
    ValidationResult::success()
}

/// 分类名称验证
#[allow(dead_code)]
pub fn validate_category_name(name: &str) -> ValidationResult {
    if name.trim().is_empty() {
        return ValidationResult::error("name", "分类名称不能为空");
    }
    if name.len() > 100 {
        return ValidationResult::error("name", "分类名称长度不能超过 100 个字符");
    }
    ValidationResult::success()
}

/// 分页参数验证
#[allow(dead_code)]
pub fn validate_pagination(page: Option<i32>, page_size: Option<i32>) -> ValidationResult {
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(20);

    if page < 1 {
        return ValidationResult::error("page", "页码必须大于 0");
    }
    if !(1..=100).contains(&page_size) {
        return ValidationResult::error("page_size", "每页数量必须在 1-100 之间");
    }
    ValidationResult::success()
}

/// UUID 验证
#[allow(dead_code)]
pub fn validate_uuid(uuid_str: &str) -> ValidationResult {
    if Uuid::parse_str(uuid_str).is_err() {
        return ValidationResult::error("id", "无效的 ID 格式");
    }
    ValidationResult::success()
}

/// 扩展名验证
#[allow(dead_code)]
pub fn validate_extension(filename: &str, allowed: &[String]) -> ValidationResult {
    let ext = filename.rsplit('.').next().unwrap_or("");
    if ext.is_empty() {
        return ValidationResult::error("filename", "文件必须有扩展名");
    }
    if !allowed.iter().any(|a| a.eq_ignore_ascii_case(ext)) {
        return ValidationResult::error(
            "filename",
            format!("不支持的文件类型，支持的类型: {}", allowed.join(", "))
        );
    }
    ValidationResult::success()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_username_valid() {
        let result = validate_username("user_123");
        assert!(result.valid);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_validate_username_too_short() {
        let result = validate_username("ab");
        assert!(!result.valid);
        assert!(result.errors.contains_key("username"));
        assert_eq!(result.errors["username"], "用户名长度至少为 3 个字符");
    }

    #[test]
    fn test_validate_username_too_long() {
        let result = validate_username(&"a".repeat(51));
        assert!(!result.valid);
        assert_eq!(result.errors["username"], "用户名长度不能超过 50 个字符");
    }

    #[test]
    fn test_validate_username_invalid_chars() {
        let result = validate_username("user@name");
        assert!(!result.valid);
        assert_eq!(result.errors["username"], "用户名只能包含字母、数字、下划线和连字符");
    }

    #[test]
    fn test_validate_password_valid() {
        let result = validate_password("secure123");
        assert!(result.valid);
    }

    #[test]
    fn test_validate_password_too_short() {
        let result = validate_password("123");
        assert!(!result.valid);
        assert_eq!(result.errors["password"], "密码长度至少为 6 个字符");
    }

    #[test]
    fn test_validate_password_too_long() {
        let result = validate_password(&"a".repeat(129));
        assert!(!result.valid);
        assert_eq!(result.errors["password"], "密码长度不能超过 128 个字符");
    }

    #[test]
    fn test_validate_filename_valid() {
        let result = validate_filename("image_001.jpg");
        assert!(result.valid);
    }

    #[test]
    fn test_validate_filename_empty() {
        let result = validate_filename("");
        assert!(!result.valid);
        assert_eq!(result.errors["filename"], "文件名不能为空");
    }

    #[test]
    fn test_validate_filename_too_long() {
        let result = validate_filename(&"a".repeat(256));
        assert!(!result.valid);
        assert_eq!(result.errors["filename"], "文件名长度不能超过 255 个字符");
    }

    #[test]
    fn test_validate_filename_path_traversal() {
        let result = validate_filename("../../../etc/passwd");
        assert!(!result.valid);
        assert_eq!(result.errors["filename"], "文件名包含非法字符");
    }

    #[test]
    fn test_validate_filename_reserved_name() {
        let result = validate_filename("CON");
        assert!(!result.valid);
        assert_eq!(result.errors["filename"], "文件名是系统保留名称");
    }

    #[test]
    fn test_validate_filename_reserved_name_case_insensitive() {
        let result = validate_filename("con");
        assert!(!result.valid);
    }

    #[test]
    fn test_validate_category_name_valid() {
        let result = validate_category_name("Nature");
        assert!(result.valid);
    }

    #[test]
    fn test_validate_category_name_whitespace_only() {
        let result = validate_category_name("   ");
        assert!(!result.valid);
        assert_eq!(result.errors["name"], "分类名称不能为空");
    }

    #[test]
    fn test_validate_category_name_too_long() {
        let result = validate_category_name(&"a".repeat(101));
        assert!(!result.valid);
        assert_eq!(result.errors["name"], "分类名称长度不能超过 100 个字符");
    }

    #[test]
    fn test_validate_pagination_valid() {
        let result = validate_pagination(Some(1), Some(20));
        assert!(result.valid);
    }

    #[test]
    fn test_validate_pagination_invalid_page() {
        let result = validate_pagination(Some(0), Some(20));
        assert!(!result.valid);
        assert_eq!(result.errors["page"], "页码必须大于 0");
    }

    #[test]
    fn test_validate_pagination_invalid_page_size() {
        let result = validate_pagination(Some(1), Some(0));
        assert!(!result.valid);
        assert_eq!(result.errors["page_size"], "每页数量必须在 1-100 之间");
    }

    #[test]
    fn test_validate_pagination_default_values() {
        let result = validate_pagination(None, None);
        assert!(result.valid);
    }

    #[test]
    fn test_validate_uuid_valid() {
        let uuid = Uuid::new_v4().to_string();
        let result = validate_uuid(&uuid);
        assert!(result.valid);
    }

    #[test]
    fn test_validate_uuid_invalid() {
        let result = validate_uuid("not-a-uuid");
        assert!(!result.valid);
        assert_eq!(result.errors["id"], "无效的 ID 格式");
    }

    #[test]
    fn test_validate_extension_valid() {
        let allowed = vec!["jpg".to_string(), "png".to_string()];
        let result = validate_extension("photo.JPG", &allowed);
        assert!(result.valid);
    }

    #[test]
    fn test_validate_extension_missing() {
        let allowed = vec!["jpg".to_string()];
        // 文件名以点结尾，扩展名为空
        let result = validate_extension("file.", &allowed);
        assert!(!result.valid);
        assert_eq!(result.errors["filename"], "文件必须有扩展名");
    }

    #[test]
    fn test_validate_extension_no_dot() {
        let allowed = vec!["jpg".to_string()];
        // 文件名没有点，整个字符串作为扩展名检查
        let result = validate_extension("noextension", &allowed);
        assert!(!result.valid);
        assert!(result.errors["filename"].contains("不支持的文件类型"));
    }

    #[test]
    fn test_validate_extension_unsupported() {
        let allowed = vec!["jpg".to_string(), "png".to_string()];
        let result = validate_extension("video.mp4", &allowed);
        assert!(!result.valid);
        assert!(result.errors["filename"].contains("不支持的文件类型"));
    }

    #[test]
    fn test_validation_result_success() {
        let result = ValidationResult::success();
        assert!(result.valid);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_validation_result_error() {
        let result = ValidationResult::error("field", "error message");
        assert!(!result.valid);
        assert_eq!(result.errors["field"], "error message");
    }
}

