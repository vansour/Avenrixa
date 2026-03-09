//! 认证领域服务
//!
//! 小封装用户登录、修改密码等业务逻辑

use tracing::info;
use uuid::Uuid;

use super::repository::AuthRepository;
use super::service::AuthService;
use crate::error::AppError;
use crate::models::{LoginRequest, UpdateProfileRequest, User, UserResponse};

/// 认证领域服务
pub struct AuthDomainService<R: AuthRepository> {
    repository: R,
    auth_service: AuthService,
}

impl<R: AuthRepository> AuthDomainService<R> {
    pub fn new(repository: R, auth_service: AuthService) -> Self {
        Self {
            repository,
            auth_service,
        }
    }

    /// 用户登录
    #[tracing::instrument(skip(self, req), fields(username = %req.username))]
    pub async fn login(&self, req: LoginRequest) -> Result<(UserResponse, String), AppError> {
        // 1. 查找用户
        let user = self
            .repository
            .find_user_by_username(&req.username)
            .await?
            .ok_or(AppError::InvalidPassword)?;

        // 2. 验证密码
        let is_valid = AuthService::verify_password(&req.password, &user.password_hash)?;
        if !is_valid {
            return Err(AppError::InvalidPassword);
        }

        // 3. 生成访问令牌（业务鉴权使用）
        // 注意：refresh token 仅用于刷新流程，不能直接作为业务访问令牌
        let access_token = self
            .auth_service
            .generate_token(user.id, &user.username, &user.role)?;

        info!("User logged in: {}", user.username);

        Ok((
            UserResponse {
                id: user.id,
                username: user.username,
                role: user.role,
                created_at: user.created_at,
            },
            access_token,
        ))
    }

    /// 修改密码
    pub async fn change_password(
        &self,
        user_id: Uuid,
        req: UpdateProfileRequest,
    ) -> Result<(), AppError> {
        // 1. 查找用户
        let user = self
            .repository
            .find_user_by_id(user_id)
            .await?
            .ok_or(AppError::UserNotFound)?;

        // 2. 验证当前密码
        let is_valid = AuthService::verify_password(&req.current_password, &user.password_hash)?;
        if !is_valid {
            return Err(AppError::InvalidPassword);
        }

        // 3. 验证新密码
        if let Some(new_password) = req.new_password {
            if new_password.len() < 6 || new_password.len() > 100 {
                return Err(AppError::InvalidPasswordLength);
            }

            // 4. 生成新密码哈希
            let new_hash = AuthService::hash_password(&new_password)?;

            // 5. 更新密码
            self.repository
                .update_user_password(user_id, &new_hash)
                .await?;

            info!("Password changed for user_id: {}", user_id);
        }

        Ok(())
    }

    /// 获取当前用户
    pub async fn get_current_user(&self, user_id: Uuid) -> Result<UserResponse, AppError> {
        let user = self
            .repository
            .find_user_by_id(user_id)
            .await?
            .ok_or(AppError::UserNotFound)?;

        Ok(UserResponse {
            id: user.id,
            username: user.username,
            role: user.role,
            created_at: user.created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::mock_repository::MockAuthRepository;
    use super::super::service::AuthService;
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn test_login_success() {
        // 设置测试环境变量
        unsafe {
            std::env::set_var("JWT_SECRET", "test-secret-key-at-least-32-characters-long");
        }

        let repo = MockAuthRepository::new();
        let config = Config::default();
        let auth_service = AuthService::new(&config).unwrap();

        // 创建测试用户
        let password_hash = AuthService::hash_password("password").unwrap();
        let test_user = crate::models::User {
            id: uuid::Uuid::new_v4(),
            username: "admin".to_string(),
            password_hash,
            role: "admin".to_string(),
            created_at: chrono::Utc::now(),
        };
        repo.users.lock().unwrap().push(test_user);

        let service = AuthDomainService::new(repo, auth_service);

        let login_req = LoginRequest {
            username: "admin".to_string(),
            password: "password".to_string(),
        };

        let (res, _) = service.login(login_req).await.unwrap();
        assert_eq!(res.username, "admin");
    }

    #[tokio::test]
    async fn test_login_wrong_password() {
        unsafe {
            std::env::set_var("JWT_SECRET", "test-secret-key-at-least-32-characters-long");
        }
        let repo = MockAuthRepository::new();
        let config = Config::default();
        let auth_service = AuthService::new(&config).unwrap();
        let service = AuthDomainService::new(repo, auth_service);

        let login_req = LoginRequest {
            username: "admin".to_string(),
            password: "wrongpassword".to_string(),
        };

        let res = service.login(login_req).await;
        assert!(matches!(res, Err(AppError::InvalidPassword)));
    }
}
