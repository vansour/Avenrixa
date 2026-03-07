//! 认证领域服务
//!
//! 小封装用户注册、登录、密码重置等业务逻辑

use chrono::{Duration, Utc};
use tracing::info;
use uuid::Uuid;

use super::claims::Claims;
use super::repository::{AuthRepository, PasswordResetToken};
use super::service::AuthService;
use crate::email::MailService;
use crate::error::AppError;
use crate::models::{User, RegisterRequest, LoginRequest, AuthResponse, UserResponse};

/// 认证领域服务
pub struct AuthDomainService<R: AuthRepository> {
    repository: R,
    auth_service: AuthService,
    mail_service: Option<MailService>,
}

impl<R: AuthRepository> AuthDomainService<R> {
    pub fn new(repository: R, auth_service: AuthService) -> Self {
        Self {
            repository,
            auth_service,
            mail_service: None,
        }
    }

    pub fn with_mail_service(mut self, mail_service: MailService) -> Self {
        self.mail_service = Some(mail_service);
        self
    }

    /// 用户注册
    #[tracing::instrument(skip(self, req), fields(username = %req.username))]
    pub async fn register(&self, req: RegisterRequest) -> Result<AuthResponse, AppError> {
        // 1. 验证用户名
        if req.username.len() < 3 || req.username.len() > 50 {
            return Err(AppError::InvalidUsernameLength);
        }

        // 2. 验证密码
        if req.password.len() < 6 {
            return Err(AppError::InvalidPasswordLength);
        }

        // 3. 生成密码哈希
        let password_hash = AuthService::hash_password(&req.password)?;

        // 4. 创建用户
        let user_id = Uuid::new_v4();
        let user = User {
            id: user_id,
            username: req.username.clone(),
            password_hash,
            role: "user".to_string(),
            created_at: Utc::now(),
        };

        // 5. 保存到数据库
        self.repository.create_user(&user).await?;

        // 6. 生成令牌
        let access_token = self.auth_service.generate_access_token(user_id, &user.username, &user.role)?;
        let refresh_token = self.auth_service.generate_refresh_token(user_id)?;

        info!("User registered: {}", user.username);

        Ok(AuthResponse {
            access_token,
            refresh_token,
            expires_in: 900,
            user: user.into(),
        })
    }

    /// 用户登录
    #[tracing::instrument(skip(self, req), fields(username = %req.username))]
    pub async fn login(&self, req: LoginRequest) -> Result<(AuthResponse, String), AppError> {
        // 1. 查找用户
        let user = self.repository.find_user_by_username(&req.username).await?
            .ok_or(AppError::InvalidPassword)?;

        // 2. 验证密码
        let is_valid = AuthService::verify_password(&req.password, &user.password_hash)?;
        if !is_valid {
            return Err(AppError::InvalidPassword);
        }

        // 3. 生成令牌
        let access_token = self.auth_service.generate_access_token(user.id, &user.username, &user.role)?;
        let refresh_token = self.auth_service.generate_refresh_token(user.id)?;

        info!("User logged in: {}", user.username);

        Ok((
            AuthResponse {
                access_token,
                refresh_token: refresh_token.clone(),
                expires_in: 900,
                user: user.into(),
            },
            refresh_token,
        ))
    }

    /// 忘记密码
    pub async fn forgot_password(&self, email: String) -> Result<(), AppError> {
        // 1. 查找用户
        let user = self.repository.find_user_by_username(&email).await?
            .ok_or(AppError::UserNotFound)?;

        // 2. 生成重置令牌
        let reset_token = AuthService::generate_reset_token();
        let expires_at = Utc::now() + Duration::hours(1);
        let token_id = Uuid::new_v4();

        let token = PasswordResetToken {
            id: token_id,
            user_id: user.id,
            token: reset_token.clone(),
            expires_at,
            used_at: None,
            created_at: Utc::now(),
        };

        // 3. 保存令牌
        self.repository.create_password_reset_token(&token).await?;

        info!("Password reset token created for user: {}", user.username);

        // 4. 发送邮件
        if let Some(mail_service) = &self.mail_service {
            mail_service.send_password_reset_email(&user.username, &reset_token).await?;
        }

        Ok(())
    }

    /// 重置密码
    pub async fn reset_password(&self, token: String, new_password: String) -> Result<(), AppError> {
        // 1. 验证密码长度
        if new_password.len() < 6 || new_password.len() > 100 {
            return Err(AppError::InvalidPasswordLength);
        }

        // 2. 查找有效的重置令牌
        let reset_data = self.repository.find_password_reset_token(&token).await?
            .ok_or(AppError::ResetTokenInvalid)?;

        // 3. 检查令牌是否已使用
        if reset_data.used_at.is_some() {
            return Err(AppError::ResetTokenExpired);
        }

        // 4. 检查令牌是否过期
        if reset_data.expires_at < Utc::now() {
            return Err(AppError::ResetTokenExpired);
        }

        // 5. 生成新密码哈希
        let new_hash = AuthService::hash_password(&new_password)?;

        // 6. 更新密码
        self.repository.update_user_password(reset_data.user_id, &new_hash).await?;

        // 7. 标记令牌已使用
        self.repository.mark_token_used(reset_data.id).await?;

        info!("Password reset for user_id: {}", reset_data.user_id);

        Ok(())
    }

    /// 修改密码
    pub async fn change_password(
        &self,
        user_id: Uuid,
        current_password: String,
        new_password: String,
    ) -> Result<(), AppError> {
        // 1. 查找用户
        let user = self.repository.find_user_by_id(user_id).await?
            .ok_or(AppError::UserNotFound)?;

        // 2. 验证当前密码
        let is_valid = AuthService::verify_password(&current_password, &user.password_hash)?;
        if !is_valid {
            return Err(AppError::InvalidPassword);
        }

        // 3. 验证新密码
        if new_password.len() < 6 || new_password.len() > 100 {
            return Err(AppError::InvalidPasswordLength);
        }

        // 4. 生成新密码哈希
        let new_hash = AuthService::hash_password(&new_password)?;

        // 5. 更新密码
        self.repository.update_user_password(user_id, &new_hash).await?;

        info!("Password changed for user_id: {}", user_id);

        Ok(())
    }

    /// 刷新令牌
    pub async fn refresh_token(&self, refresh_token: String) -> Result<AuthResponse, AppError> {
        // 1. 验证刷新令牌
        let user_id = self.auth_service.verify_refresh_token(&refresh_token)?;

        // 2. 查找用户
        let user = self.repository.find_user_by_id(user_id).await?
            .ok_or(AppError::UserNotFound)?;

        // 3. 生成新令牌
        let access_token = self.auth_service.generate_access_token(user_id, &user.username, &user.role)?;
        let new_refresh_token = self.auth_service.generate_refresh_token(user_id)?;

        info!("Token refreshed for user: {}", user.username);

        Ok(AuthResponse {
            access_token,
            refresh_token: new_refresh_token,
            expires_in: 900,
            user: user.into(),
        })
    }

    /// 获取当前用户
    pub async fn get_current_user(&self, user_id: Uuid) -> Result<UserResponse, AppError> {
        let user = self.repository.find_user_by_id(user_id).await?
            .ok_or(AppError::UserNotFound)?;

        Ok(user.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::mock_repository::MockAuthRepository;
    use crate::config::Config;
    use super::super::service::AuthService;

    #[tokio::test]
    async fn test_register_success() {
        let repo = MockAuthRepository::new();
        let config = Config::default();
        let auth_service = AuthService::new(&config).unwrap();
        let service = AuthDomainService::new(repo, auth_service);

        let req = RegisterRequest {
            username: "testuser".to_string(),
            password: "password123".to_string(),
        };

        let res = service.register(req).await.unwrap();
        assert_eq!(res.user.username, "testuser");
    }

    #[tokio::test]
    async fn test_login_success() {
        let repo = MockAuthRepository::new();
        let config = Config::default();
        let auth_service = AuthService::new(&config).unwrap();
        let service = AuthDomainService::new(repo, auth_service);

        // 先注册
        let reg_req = RegisterRequest {
            username: "loginuser".to_string(),
            password: "password123".to_string(),
        };
        service.register(reg_req).await.unwrap();

        // 再登录
        let login_req = LoginRequest {
            username: "loginuser".to_string(),
            password: "password123".to_string(),
        };
        let (res, _) = service.login(login_req).await.unwrap();
        assert_eq!(res.user.username, "loginuser");
    }

    #[tokio::test]
    async fn test_login_wrong_password() {
        let repo = MockAuthRepository::new();
        let config = Config::default();
        let auth_service = AuthService::new(&config).unwrap();
        let service = AuthDomainService::new(repo, auth_service);

        let reg_req = RegisterRequest {
            username: "wrongpass".to_string(),
            password: "password123".to_string(),
        };
        service.register(reg_req).await.unwrap();

        let login_req = LoginRequest {
            username: "wrongpass".to_string(),
            password: "wrongpassword".to_string(),
        };
        let res = service.login(login_req).await;
        assert!(matches!(res, Err(AppError::InvalidPassword)));
    }
}
