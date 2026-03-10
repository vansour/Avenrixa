use super::common::AuthDomainService;
use crate::domain::auth::repository::AuthRepository;
use crate::domain::auth::service::AuthService;
use crate::error::AppError;
use crate::models::LoginRequest;
use lettre::Address;
use tracing::info;

impl<R: AuthRepository> AuthDomainService<R> {
    /// 用户登录
    #[tracing::instrument(skip(self, req), fields(email = %req.email))]
    pub async fn login(&self, req: LoginRequest) -> Result<crate::models::UserResponse, AppError> {
        let email = req.email.trim().to_lowercase();
        if email.is_empty() || email.parse::<Address>().is_err() {
            return Err(AppError::InvalidPassword);
        }

        let user = self
            .repository
            .find_user_by_email(&email)
            .await?
            .ok_or(AppError::InvalidPassword)?;

        if user.email_verified_at.is_none() {
            return Err(AppError::EmailNotVerified);
        }

        let is_valid = AuthService::verify_password(&req.password, &user.password_hash)?;
        if !is_valid {
            return Err(AppError::InvalidPassword);
        }

        info!("User logged in: {}", email);
        Ok(Self::to_user_response(user))
    }
}
