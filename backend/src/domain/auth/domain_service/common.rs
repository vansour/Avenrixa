use crate::domain::auth::repository::AuthRepository;
use crate::models::{User, UserResponse};

#[derive(Debug, Clone)]
pub struct PasswordResetDispatch {
    pub user_id: uuid::Uuid,
    pub username: String,
    pub email: String,
    pub token: String,
}

/// 认证领域服务
pub struct AuthDomainService<R: AuthRepository> {
    pub(super) repository: R,
}

impl<R: AuthRepository> AuthDomainService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub(super) fn to_user_response(user: User) -> UserResponse {
        UserResponse {
            id: user.id,
            username: user.username,
            role: user.role,
            created_at: user.created_at,
        }
    }
}
