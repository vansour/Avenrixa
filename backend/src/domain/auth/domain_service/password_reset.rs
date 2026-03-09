use chrono::{Duration, Utc};

use super::common::{AuthDomainService, PasswordResetDispatch};
use crate::domain::auth::repository::{AuthRepository, PasswordResetStatus};
use crate::domain::auth::service::AuthService;
use crate::error::AppError;

const PASSWORD_RESET_TTL_MINUTES: i64 = 60;

fn hash_reset_token(token: &str) -> String {
    blake3::hash(token.as_bytes()).to_hex().to_string()
}

impl<R: AuthRepository> AuthDomainService<R> {
    pub async fn request_password_reset(
        &self,
        identity: &str,
    ) -> Result<Option<PasswordResetDispatch>, AppError> {
        let identity = identity.trim();
        if identity.is_empty() {
            return Err(AppError::ValidationError(
                "用户名或邮箱不能为空".to_string(),
            ));
        }

        let Some(user) = self.repository.find_user_by_identity(identity).await? else {
            return Ok(None);
        };
        let Some(email) = user.email.clone().filter(|value| !value.trim().is_empty()) else {
            return Ok(None);
        };

        let token = AuthService::generate_reset_token();
        let token_hash = hash_reset_token(&token);
        let expires_at = Utc::now() + Duration::minutes(PASSWORD_RESET_TTL_MINUTES);

        self.repository
            .store_password_reset_token(user.id, &token_hash, expires_at)
            .await?;

        Ok(Some(PasswordResetDispatch {
            user_id: user.id,
            username: user.username,
            email,
            token,
        }))
    }

    pub async fn reset_password_by_token(
        &self,
        token: &str,
        new_password: &str,
    ) -> Result<crate::models::UserResponse, AppError> {
        if !AuthService::is_reset_token_strong(token) {
            return Err(AppError::ResetTokenInvalid);
        }
        if !(6..=100).contains(&new_password.len()) {
            return Err(AppError::InvalidPasswordLength);
        }

        let password_hash = AuthService::hash_password(new_password)?;
        let token_hash = hash_reset_token(token);
        match self
            .repository
            .reset_password_by_token(&token_hash, &password_hash)
            .await?
        {
            PasswordResetStatus::Applied(user) => Ok(Self::to_user_response(user)),
            PasswordResetStatus::Expired => Err(AppError::ResetTokenExpired),
            PasswordResetStatus::Invalid => Err(AppError::ResetTokenInvalid),
        }
    }
}
