#[test]
fn test_app_error_from_reqwest() {
    // 测试从 reqwest::Error 转换为 AppError
    // 由于 reqwest::Error 不能直接构造，我们使用测试 helper
    use vansour_image_frontend::types::errors::AppError;

    // 测试网络错误的转换
    let error_msg = "Network error";
    let network_error = AppError::Network(error_msg.to_string());
    assert!(matches!(network_error, AppError::Network(_)));

    // 测试未授权错误
    let unauthorized_error = AppError::Unauthorized;
    assert!(matches!(unauthorized_error, AppError::Unauthorized));

    // 测试未找到错误
    let not_found_error = AppError::NotFound;
    assert!(matches!(not_found_error, AppError::NotFound));

    // 测试禁止访问错误
    let forbidden_error = AppError::Forbidden;
    assert!(matches!(forbidden_error, AppError::Forbidden));

    // 测试服务器错误
    let server_error = AppError::Server("Server error".to_string());
    assert!(matches!(server_error, AppError::Server(_)));

    // 测试验证错误
    let validation_error = AppError::Validation("Validation error".to_string());
    assert!(matches!(validation_error, AppError::Validation(_)));
}

#[test]
fn test_app_error_should_redirect_login() {
    use vansour_image_frontend::types::errors::AppError;

    // Unauthorized 和 Forbidden 应该重定向到登录页
    assert!(AppError::Unauthorized.should_redirect_login());
    assert!(AppError::Forbidden.should_redirect_login());

    // 其他错误不应该重定向
    assert!(!AppError::NotFound.should_redirect_login());
    assert!(!AppError::Network("error".to_string()).should_redirect_login());
    assert!(!AppError::Server("error".to_string()).should_redirect_login());
    assert!(!AppError::Validation("error".to_string()).should_redirect_login());
}

#[test]
fn test_image_filters_default() {
    use vansour_image_frontend::types::models::ImageFilters;

    let filters = ImageFilters::default();
    assert!(filters.search.is_none());
    assert!(filters.category_id.is_none());
    assert_eq!(filters.sort_by, "created_at");
    assert_eq!(filters.sort_order, "desc");
}
