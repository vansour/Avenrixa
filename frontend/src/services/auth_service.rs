use crate::services::api_client::ApiClient;
use crate::store::auth::AuthStore;
use crate::types::api::{LoginRequest, UpdateProfileRequest, UserResponse};
use crate::types::errors::Result;

/// 认证服务
#[derive(Clone)]
pub struct AuthService {
    api_client: ApiClient,
    auth_store: AuthStore,
}

impl AuthService {
    pub fn new(api_client: ApiClient, auth_store: AuthStore) -> Self {
        Self {
            api_client,
            auth_store,
        }
    }

    /// 登录
    pub async fn login(&self, req: LoginRequest) -> Result<UserResponse> {
        let response_text = self
            .api_client
            .post_json("/api/v1/auth/login", &req)
            .await?;
        let user: UserResponse = serde_json::from_str(&response_text).map_err(|e| {
            crate::types::errors::AppError::Server(format!("解析用户响应失败: {}", e))
        })?;

        // 登录后仅同步当前用户，认证 Cookie 由浏览器维护
        self.auth_store.set_user(user.clone());
        Ok(user)
    }

    /// 获取当前用户
    pub async fn get_me(&self) -> Result<UserResponse> {
        self.api_client.get_json("/api/v1/auth/me").await
    }

    /// 登出
    pub async fn logout(&self) -> Result<()> {
        self.api_client
            .post("/api/v1/auth/logout", String::new())
            .await?;
        self.auth_store.logout();
        Ok(())
    }

    /// 修改密码
    pub async fn change_password(&self, req: UpdateProfileRequest) -> Result<()> {
        self.api_client
            .post_json("/api/v1/auth/change-password", &req)
            .await?;
        Ok(())
    }
}
