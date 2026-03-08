use reqwest::Client;
use crate::store::auth::AuthStore;
use crate::types::api::{LoginRequest, UserResponse};
use crate::types::errors::Result;

/// 认证服务（简化版）
pub struct AuthService {
    client: Client,
    auth_store: AuthStore,
}

impl AuthService {
    pub fn new(client: Client, auth_store: AuthStore) -> Self {
        Self { client, auth_store }
    }

    /// 登录
    pub async fn login(&self, _req: LoginRequest) -> Result<()> {
        self.auth_store.login(
            UserResponse {
                id: uuid::Uuid::new_v4(),
                username: "test_user".to_string(),
                role: "user".to_string(),
                created_at: chrono::Utc::now(),
            },
            "test_token".to_string(),
        );
        Ok(())
    }
}
