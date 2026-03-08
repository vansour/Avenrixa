#[test]
fn test_api_client_url() {
    use vansour_image_frontend::services::api_client::ApiClient;
    use vansour_image_frontend::store::auth::AuthStore;

    let auth_store = AuthStore::new();
    let client = ApiClient::new("http://localhost:8000".to_string(), auth_store);

    assert_eq!(client.url("/api/test"), "http://localhost:8000/api/test");
}

#[test]
fn test_app_error_network() {
    use vansour_image_frontend::types::errors::AppError;

    let error = AppError::Network("Connection failed".to_string());
    assert!(matches!(error, AppError::Network(_)));
}

#[test]
fn test_app_error_unauthorized() {
    use vansour_image_frontend::types::errors::AppError;

    let error = AppError::Unauthorized;
    assert!(matches!(error, AppError::Unauthorized));
    assert!(error.should_redirect_login());
}

#[test]
fn test_app_error_not_found() {
    use vansour_image_frontend::types::errors::AppError;

    let error = AppError::NotFound;
    assert!(matches!(error, AppError::NotFound));
}

#[test]
fn test_app_error_server() {
    use vansour_image_frontend::types::errors::AppError;

    let error = AppError::Server("Server error".to_string());
    assert!(matches!(error, AppError::Server(_)));
}
