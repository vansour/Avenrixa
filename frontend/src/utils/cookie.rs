/// Cookie 工具函数
///
/// 从 Cookie header 中解析 auth_token
///
/// # 示例
/// ```
/// use frontend_lib::utils::parse_auth_token;
///
/// let cookie = "auth_token=xyz123; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=604800";
/// assert_eq!(parse_auth_token(cookie), Some("xyz123".to_string()));
/// ```
///
/// # 解析逻辑
/// 1. 按 `; ` 分割 Cookie 字符串
/// 2. 对每个部分，按 `=` 分割键值
/// 3. 查找 `auth_token` 键
/// 4. 提取 token 值，跳过属性部分
pub fn parse_auth_token(cookie_header: &str) -> Option<String> {
    for cookie_pair in cookie_header.split(';') {
        let cookie_pair = cookie_pair.trim();
        if let Some((key, value)) = cookie_pair.split_once('=') {
            let key = key.trim();
            if key == "auth_token" {
                // 分割属性部分（HttpOnly、Secure、SameSite、Path、Max-Age）
                let token = value.split(';').next().unwrap_or(value).trim();
                if !token.is_empty() {
                    return Some(token.to_string());
                }
            }
        }
    }
    None
}

/// 构建 HttpOnly auth_token Cookie
///
/// # 参数
/// - `token`: 认证 token
/// - `max_age_days`: Cookie 有效期（天）
///
/// # 示例
/// ```
/// use frontend_lib::utils::build_auth_cookie;
///
/// let cookie = build_auth_cookie("token123", 7);
/// assert!(cookie.contains("auth_token=token123"));
/// assert!(cookie.contains("Max-Age=604800"));
/// ```
pub fn build_auth_cookie(token: &str, max_age_days: u32) -> String {
    let max_age = max_age_days * 24 * 3600; // 转换为秒
    format!(
        "auth_token={}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={}",
        token, max_age
    )
}

/// 从 Set-Cookie header 中提取 auth_token
///
/// 当后端返回新的 Cookie 时，我们需要从响应中提取它
pub fn extract_auth_token_from_set_cookie(set_cookie_header: &str) -> Option<String> {
    for cookie_pair in set_cookie_header.split(';') {
        let cookie_pair = cookie_pair.trim();
        if let Some((key, value)) = cookie_pair.split_once('=') {
            let key = key.trim();
            if key == "auth_token" {
                let token = value.split(';').next().unwrap_or(value).trim();
                if !token.is_empty() {
                    return Some(token.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_auth_token_valid() {
        let cookie_header =
            "auth_token=test123; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=604800";
        assert_eq!(parse_auth_token(cookie_header), Some("test123".to_string()));
    }

    #[test]
    fn test_parse_auth_token_empty() {
        let cookie_header = "other=123; auth_token=xyz; HttpOnly; Secure";
        assert_eq!(parse_auth_token(cookie_header), Some("xyz".to_string()));
    }

    #[test]
    fn test_parse_auth_token_not_found() {
        let cookie_header = "other=123; HttpOnly; Secure";
        assert!(parse_auth_token(cookie_header).is_none());
    }

    #[test]
    fn test_build_auth_cookie() {
        let cookie = build_auth_cookie("token123", 7);
        assert!(cookie.contains("auth_token=token123"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("Secure"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains("Max-Age=604800"));
    }
}
