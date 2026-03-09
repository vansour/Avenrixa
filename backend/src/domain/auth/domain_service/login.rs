use super::common::AuthDomainService;
use crate::domain::auth::repository::AuthRepository;
use crate::domain::auth::service::AuthService;
use crate::error::AppError;
use crate::models::LoginRequest;
use tracing::info;

impl<R: AuthRepository> AuthDomainService<R> {
    /// 用户登录
    #[tracing::instrument(skip(self, req), fields(username = %req.username))]
    pub async fn login(&self, req: LoginRequest) -> Result<crate::models::UserResponse, AppError> {
        let user = self
            .repository
            .find_user_by_username(&req.username)
            .await?
            .ok_or(AppError::InvalidPassword)?;

        let is_valid = AuthService::verify_password(&req.password, &user.password_hash)?;
        if !is_valid {
            return Err(AppError::InvalidPassword);
        }

        info!("User logged in: {}", user.username);
        Ok(Self::to_user_response(user))
    }
}
