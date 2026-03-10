use super::common::AuthDomainService;
use crate::domain::auth::repository::AuthRepository;
use crate::domain::auth::service::AuthService;
use crate::error::AppError;
use crate::models::UpdateProfileRequest;
use tracing::info;
use uuid::Uuid;

impl<R: AuthRepository> AuthDomainService<R> {
    /// 修改密码
    pub async fn change_password(
        &self,
        user_id: Uuid,
        req: UpdateProfileRequest,
    ) -> Result<(), AppError> {
        let user = self
            .repository
            .find_user_by_id(user_id)
            .await?
            .ok_or(AppError::UserNotFound)?;

        let is_valid = AuthService::verify_password(&req.current_password, &user.password_hash)?;
        if !is_valid {
            return Err(AppError::InvalidPassword);
        }

        if let Some(new_password) = req.new_password {
            if user.role.eq_ignore_ascii_case("admin") && new_password.len() < 12 {
                return Err(AppError::ValidationError(
                    "管理员密码至少需要 12 个字符".to_string(),
                ));
            }

            if new_password.len() < 6 {
                return Err(AppError::InvalidPasswordLength);
            }

            if new_password.len() > 100 {
                return Err(AppError::InvalidPasswordLength);
            }

            let new_hash = AuthService::hash_password(&new_password)?;
            self.repository
                .update_user_password(user_id, &new_hash)
                .await?;

            info!("Password changed for user_id: {}", user_id);
        }

        Ok(())
    }

    /// 获取当前用户
    pub async fn get_current_user(
        &self,
        user_id: Uuid,
    ) -> Result<crate::models::UserResponse, AppError> {
        let user = self
            .repository
            .find_user_by_id(user_id)
            .await?
            .ok_or(AppError::UserNotFound)?;

        Ok(Self::to_user_response(user))
    }
}
