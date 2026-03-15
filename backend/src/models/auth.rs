use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum UserRole {
    Admin,
    User,
    Unknown,
}

impl UserRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::User => "user",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "admin" => Self::Admin,
            "user" => Self::User,
            _ => Self::Unknown,
        }
    }

    pub fn is_admin(self) -> bool {
        matches!(self, Self::Admin)
    }
}

impl From<String> for UserRole {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<UserRole> for String {
    fn from(value: UserRole) -> Self {
        value.as_str().to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[sqlx(default)]
    pub email_verified_at: Option<DateTime<Utc>>,
    pub password_hash: String,
    #[sqlx(try_from = "String")]
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub current_password: String,
    pub new_password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PasswordResetRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct PasswordResetConfirmRequest {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct EmailVerificationConfirmRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    #[serde(skip_serializing)]
    pub id: Uuid,
    pub email: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            role: user.role,
            created_at: user.created_at,
        }
    }
}

impl axum::response::IntoResponse for UserResponse {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        axum::Json(self).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AdminUserSummary {
    pub id: Uuid,
    pub email: String,
    #[sqlx(try_from = "String")]
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
}

impl From<User> for AdminUserSummary {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            role: user.role,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UserUpdateRequest {
    pub role: Option<UserRole>,
}

#[cfg(test)]
mod tests {
    use super::UserRole;

    #[test]
    fn user_role_parses_known_values_case_insensitively() {
        assert_eq!(UserRole::parse("admin"), UserRole::Admin);
        assert_eq!(UserRole::parse("USER"), UserRole::User);
    }

    #[test]
    fn user_role_falls_back_to_unknown() {
        assert_eq!(UserRole::parse("refresh"), UserRole::Unknown);
        assert_eq!(UserRole::parse("moderator"), UserRole::Unknown);
    }

    #[test]
    fn user_role_reports_admin_state() {
        assert!(UserRole::Admin.is_admin());
        assert!(!UserRole::User.is_admin());
        assert!(!UserRole::Unknown.is_admin());
    }
}
