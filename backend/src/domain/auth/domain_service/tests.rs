use super::common::AuthDomainService;
use crate::domain::auth::mock_repository::MockAuthRepository;
use crate::domain::auth::repository::AuthRepository;
use crate::domain::auth::service::AuthService;
use crate::error::AppError;
use crate::models::LoginRequest;
use uuid::Uuid;

fn build_test_user() -> crate::models::User {
    let password_hash = AuthService::hash_password("password").unwrap();
    crate::models::User {
        id: Uuid::new_v4(),
        username: "admin".to_string(),
        email: Some("admin@example.com".to_string()),
        password_hash,
        role: "admin".to_string(),
        created_at: chrono::Utc::now(),
    }
}

#[tokio::test]
async fn test_login_success() {
    unsafe {
        std::env::set_var("JWT_SECRET", "test-secret-key-at-least-32-characters-long");
    }

    let repo = MockAuthRepository::new();

    let test_user = build_test_user();
    repo.users.lock().unwrap().push(test_user);

    let service = AuthDomainService::new(repo);
    let login_req = LoginRequest {
        username: "admin".to_string(),
        password: "password".to_string(),
    };

    let res = service.login(login_req).await.unwrap();
    assert_eq!(res.username, "admin");
}

#[tokio::test]
async fn test_login_wrong_password() {
    unsafe {
        std::env::set_var("JWT_SECRET", "test-secret-key-at-least-32-characters-long");
    }
    let repo = MockAuthRepository::new();
    let service = AuthDomainService::new(repo);

    let login_req = LoginRequest {
        username: "admin".to_string(),
        password: "wrongpassword".to_string(),
    };

    let res = service.login(login_req).await;
    assert!(matches!(res, Err(AppError::InvalidPassword)));
}

#[tokio::test]
async fn test_request_password_reset_creates_dispatch() {
    let repo = MockAuthRepository::new();
    let test_user = build_test_user();
    repo.users.lock().unwrap().push(test_user.clone());
    let service = AuthDomainService::new(repo);

    let dispatch = service
        .request_password_reset(&test_user.username)
        .await
        .expect("request should succeed")
        .expect("dispatch should exist");

    assert_eq!(dispatch.user_id, test_user.id);
    assert_eq!(dispatch.email, "admin@example.com");
    assert!(dispatch.token.len() >= 32);
}

#[tokio::test]
async fn test_reset_password_by_token_updates_password() {
    let repo = MockAuthRepository::new();
    let test_user = build_test_user();
    repo.users.lock().unwrap().push(test_user.clone());
    let service = AuthDomainService::new(repo.clone());

    let dispatch = service
        .request_password_reset(&test_user.username)
        .await
        .expect("request should succeed")
        .expect("dispatch should exist");

    service
        .reset_password_by_token(&dispatch.token, "new-password-123")
        .await
        .expect("reset should succeed");

    let updated_user = repo
        .find_user_by_id(test_user.id)
        .await
        .expect("query should succeed")
        .expect("user should exist");
    assert!(
        AuthService::verify_password("new-password-123", &updated_user.password_hash)
            .expect("password verification should succeed")
    );
}
