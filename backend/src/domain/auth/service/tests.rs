use super::*;
use crate::config::Config;
use chrono::Utc;
use uuid::Uuid;

fn test_config() -> Config {
    Config::default()
}

fn create_test_service() -> AuthService {
    let config = test_config();
    unsafe {
        std::env::set_var(
            "JWT_SECRET",
            "test_secret_key_for_testing_must_be_long_enough_32chars",
        );
    }
    AuthService::new(&config).expect("Failed to create test auth service")
}

#[test]
fn test_hash_password() {
    let password = "test_password_123";
    let hash = AuthService::hash_password(password).expect("Failed to hash password");

    assert!(hash.starts_with("$2b$"));
    assert_ne!(hash, password);
}

#[test]
fn test_verify_password() {
    let password = "test_password_123";
    let hash = AuthService::hash_password(password).expect("Failed to hash password");

    assert!(AuthService::verify_password(password, &hash).expect("Failed to verify"));
    assert!(!AuthService::verify_password("wrong_password", &hash).expect("Failed to verify"));
}

#[test]
fn test_generate_and_verify_token() {
    let service = create_test_service();
    let user_id = Uuid::new_v4();
    let username = "testuser";
    let role = "user";
    let token_version = 3;

    let token = service
        .generate_token(user_id, username, role, token_version)
        .expect("Failed to generate token");

    let claims = service
        .verify_token(&token)
        .expect("Failed to verify token");
    assert_eq!(claims.sub, user_id);
    assert_eq!(claims.username, username);
    assert_eq!(claims.role, role);
    assert_eq!(claims.token_version, token_version);
}

#[test]
fn test_generate_token_uses_cookie_max_age() {
    let mut config = test_config();
    config.cookie.max_age_seconds = 2 * 3600;
    unsafe {
        std::env::set_var(
            "JWT_SECRET",
            "test_secret_key_for_testing_must_be_long_enough_32chars",
        );
    }
    let service = AuthService::new(&config).expect("Failed to create test auth service");

    let token = service
        .generate_token(Uuid::new_v4(), "testuser", "user", 0)
        .expect("Failed to generate token");
    let claims = service
        .verify_token(&token)
        .expect("Failed to verify token");

    let now = Utc::now().timestamp();
    let expected_exp = now + 2 * 3600;
    assert!((claims.exp - expected_exp).abs() <= 1);
}

#[test]
fn test_generate_reset_token() {
    let token = AuthService::generate_reset_token();
    assert_eq!(token.len(), 32);
    assert!(
        token
            .chars()
            .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit())
    );
}

#[test]
fn test_is_reset_token_strong() {
    assert!(AuthService::is_reset_token_strong(
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ123456"
    ));
    assert!(AuthService::is_reset_token_strong(&"A".repeat(64)));
    assert!(!AuthService::is_reset_token_strong("SHORT"));
    assert!(!AuthService::is_reset_token_strong(&"A".repeat(31)));
}

#[test]
fn test_generate_access_token() {
    let service = create_test_service();
    let user_id = Uuid::new_v4();

    let token = service
        .generate_access_token(user_id, "testuser", "user", 7)
        .expect("Failed to generate access token");

    let claims = service
        .verify_token(&token)
        .expect("Failed to verify token");
    assert_eq!(claims.sub, user_id);
    assert_eq!(claims.username, "testuser");
    assert_eq!(claims.role, "user");
    assert_eq!(claims.token_version, 7);

    let now = Utc::now().timestamp();
    let expected_exp = now + 15 * 60;
    assert!((claims.exp - expected_exp).abs() <= 1);
}

#[test]
fn test_generate_and_verify_refresh_token() {
    let service = create_test_service();
    let user_id = Uuid::new_v4();

    let token = service
        .generate_refresh_token(user_id, 2)
        .expect("Failed to generate refresh token");

    let result = service
        .verify_refresh_token(&token)
        .expect("Failed to verify refresh token");
    assert_eq!(result, user_id);
}

#[test]
fn test_verify_refresh_token_rejects_non_refresh_token() {
    let service = create_test_service();
    let user_id = Uuid::new_v4();

    let access_token = service
        .generate_access_token(user_id, "testuser", "user", 0)
        .expect("Failed to generate access token");

    let result = service.verify_refresh_token(&access_token);
    assert!(result.is_err());
}

#[test]
fn test_invalid_token_rejected() {
    let service = create_test_service();
    let result = service.verify_token("invalid_token");
    assert!(result.is_err());
}
