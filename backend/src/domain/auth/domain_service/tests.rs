use super::common::AuthDomainService;
use crate::domain::auth::mock_repository::MockAuthRepository;
use crate::domain::auth::repository::AuthRepository;
use crate::domain::auth::service::AuthService;
use crate::error::AppError;
use crate::models::{LoginRequest, RegisterRequest};
use uuid::Uuid;

fn build_test_user() -> crate::models::User {
    let password_hash = AuthService::hash_password("password").unwrap();
    crate::models::User {
        id: Uuid::new_v4(),
        email: "admin@example.com".to_string(),
        email_verified_at: Some(chrono::Utc::now()),
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
        email: "admin@example.com".to_string(),
        password: "password".to_string(),
    };

    let res = service.login(login_req).await.unwrap();
    assert_eq!(res.email, "admin@example.com");
}

#[tokio::test]
async fn test_login_wrong_password() {
    unsafe {
        std::env::set_var("JWT_SECRET", "test-secret-key-at-least-32-characters-long");
    }
    let repo = MockAuthRepository::new();
    let service = AuthDomainService::new(repo);

    let login_req = LoginRequest {
        email: "admin@example.com".to_string(),
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
        .request_password_reset(&test_user.email)
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
        .request_password_reset(&test_user.email)
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

#[tokio::test]
async fn test_register_creates_unverified_user_and_dispatch() {
    let repo = MockAuthRepository::new();
    let service = AuthDomainService::new(repo.clone());

    let dispatch = service
        .register(RegisterRequest {
            email: "new-user@example.com".to_string(),
            password: "password-123".to_string(),
        })
        .await
        .expect("registration should succeed");

    let created_user = repo
        .find_user_by_id(dispatch.user_id)
        .await
        .expect("query should succeed")
        .expect("user should exist");
    assert_eq!(created_user.email, "new-user@example.com");
    assert!(created_user.email_verified_at.is_none());
    assert!(!dispatch.token.is_empty());
}

#[tokio::test]
async fn test_verify_email_marks_user_verified() {
    let repo = MockAuthRepository::new();
    let service = AuthDomainService::new(repo.clone());

    let dispatch = service
        .register(RegisterRequest {
            email: "verify-me@example.com".to_string(),
            password: "password-123".to_string(),
        })
        .await
        .expect("registration should succeed");

    let verified_user = service
        .verify_email(&dispatch.token)
        .await
        .expect("verification should succeed");
    assert_eq!(verified_user.email, "verify-me@example.com");

    let stored_user = repo
        .find_user_by_id(dispatch.user_id)
        .await
        .expect("query should succeed")
        .expect("user should exist");
    assert!(stored_user.email_verified_at.is_some());
}

#[tokio::test]
async fn test_login_rejects_unverified_user() {
    let repo = MockAuthRepository::new();
    let mut user = build_test_user();
    user.email_verified_at = None;
    repo.users.lock().unwrap().push(user);

    let service = AuthDomainService::new(repo);
    let result = service
        .login(LoginRequest {
            email: "admin@example.com".to_string(),
            password: "password".to_string(),
        })
        .await;

    assert!(matches!(result, Err(AppError::EmailNotVerified)));
}
