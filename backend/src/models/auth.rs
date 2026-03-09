use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub username: Option<String>,
    pub current_password: String,
    pub new_password: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    #[serde(skip_serializing)]
    pub id: Uuid,
    pub username: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
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
    pub username: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

impl From<User> for AdminUserSummary {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            role: user.role,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UserUpdateRequest {
    pub role: Option<String>,
}
