use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
pub use shared_types::admin::AdminUserSummary;
pub use shared_types::auth::{
    EmailVerificationConfirmRequest, LoginRequest, PasswordResetConfirmRequest,
    PasswordResetRequest, RegisterRequest, UpdateProfileRequest, UserUpdateRequest,
};
pub use shared_types::common::UserRole;
use uuid::Uuid;

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
pub struct AdminUserRecord {
    pub id: Uuid,
    pub email: String,
    #[sqlx(try_from = "String")]
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
}

impl From<User> for AdminUserSummary {
    fn from(user: User) -> Self {
        Self {
            id: user.id.to_string(),
            email: user.email,
            role: user.role,
            created_at: user.created_at,
        }
    }
}

impl From<AdminUserRecord> for AdminUserSummary {
    fn from(user: AdminUserRecord) -> Self {
        Self {
            id: user.id.to_string(),
            email: user.email,
            role: user.role,
            created_at: user.created_at,
        }
    }
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
