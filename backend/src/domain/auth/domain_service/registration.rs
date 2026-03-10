use chrono::{Duration, Utc};
use lettre::Address;
use tracing::info;
use uuid::Uuid;

use super::common::{AuthDomainService, EmailVerificationDispatch};
use crate::domain::auth::repository::{AuthRepository, EmailVerificationStatus};
use crate::domain::auth::service::AuthService;
use crate::error::AppError;
use crate::models::{RegisterRequest, User};

const EMAIL_VERIFICATION_TTL_HOURS: i64 = 24;

fn hash_email_verification_token(token: &str) -> String {
    blake3::hash(format!("email-verify:{token}").as_bytes())
        .to_hex()
        .to_string()
}

fn normalize_email(email: &str) -> Result<String, AppError> {
    let normalized = email.trim().to_lowercase();
    if normalized.is_empty() {
        return Err(AppError::ValidationError("邮箱不能为空".to_string()));
    }
    normalized
        .parse::<Address>()
        .map_err(|_| AppError::ValidationError("邮箱格式无效".to_string()))?;
    Ok(normalized)
}

impl<R: AuthRepository> AuthDomainService<R> {
    pub async fn register(
        &self,
        req: RegisterRequest,
    ) -> Result<EmailVerificationDispatch, AppError> {
        let email = normalize_email(&req.email)?;
        if !(6..=100).contains(&req.password.len()) {
            return Err(AppError::InvalidPasswordLength);
        }

        let password_hash = AuthService::hash_password(&req.password)?;
        let existing_by_email = self.repository.find_user_by_email(&email).await?;

        let user = match existing_by_email {
            None => {
                let user = User {
                    id: Uuid::new_v4(),
                    email: email.clone(),
                    email_verified_at: None,
                    password_hash: password_hash.clone(),
                    role: "user".to_string(),
                    created_at: Utc::now(),
                };
                self.repository.create_user(&user).await?;
                user
            }
            Some(user) if user.email_verified_at.is_none() => {
                self.repository
                    .update_user_password(user.id, &password_hash)
                    .await?;

                User {
                    password_hash,
                    ..user
                }
            }
            Some(_) => return Err(AppError::EmailExists),
        };

        let token = AuthService::generate_reset_token();
        let token_hash = hash_email_verification_token(&token);
        let expires_at = Utc::now() + Duration::hours(EMAIL_VERIFICATION_TTL_HOURS);
        self.repository
            .store_email_verification_token(user.id, &token_hash, expires_at)
            .await?;

        info!("User registration requested: {}", email);

        Ok(EmailVerificationDispatch {
            user_id: user.id,
            email,
            token,
        })
    }

    pub async fn verify_email(&self, token: &str) -> Result<crate::models::UserResponse, AppError> {
        if !AuthService::is_reset_token_strong(token) {
            return Err(AppError::EmailVerificationInvalid);
        }

        let token_hash = hash_email_verification_token(token);
        match self.repository.verify_email_by_token(&token_hash).await? {
            EmailVerificationStatus::Applied(user) => Ok(Self::to_user_response(user)),
            EmailVerificationStatus::Expired => Err(AppError::EmailVerificationExpired),
            EmailVerificationStatus::Invalid => Err(AppError::EmailVerificationInvalid),
        }
    }
}
