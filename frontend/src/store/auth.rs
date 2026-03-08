use crate::types::api::UserResponse;
use parking_lot::RwLock;
use std::sync::Arc;

/// 认证状态管理 Store
#[derive(Clone)]
pub struct AuthStore {
    user: Arc<RwLock<Option<UserResponse>>>,
    token: Arc<RwLock<Option<String>>>,
}

impl AuthStore {
    pub fn new() -> Self {
        Self {
            user: Arc::new(RwLock::new(None)),
            token: Arc::new(RwLock::new(None)),
        }
    }

    /// 获取当前用户
    pub fn user(&self) -> Option<UserResponse> {
        self.user.read().clone()
    }

    /// 获取当前用户引用
    pub fn user_as_ref(&self) -> Option<UserResponse> {
        self.user.read().as_ref().cloned()
    }

    /// 检查用户字段是否存在
    pub fn user_is_some(&self) -> bool {
        self.user.read().is_some()
    }

    /// 检查用户是否为空
    pub fn user_is_none(&self) -> bool {
        self.user.read().is_none()
    }

    /// 检查是否已认证
    pub fn is_authenticated(&self) -> bool {
        self.user.read().is_some()
    }

    /// 登录
    pub fn login(&self, user: UserResponse, token: String) {
        *self.user.write() = Some(user);
        *self.token.write() = Some(token);
    }

    /// 登出
    pub fn logout(&self) {
        *self.user.write() = None;
        *self.token.write() = None;
    }

    /// 获取当前 token
    pub fn token(&self) -> Option<String> {
        self.token.read().clone()
    }

    /// 从 token 直接登录（用于 Cookie 认证）
    pub fn login_from_token(&self, token: &str) {
        // TODO: 需要从后端获取用户信息
        // 暂时设置一个假用户
        use chrono::Utc;
        use uuid::Uuid;

        let user = crate::types::api::UserResponse {
            id: Uuid::new_v4(),
            username: "user".to_string(),
            role: "user".to_string(),
            created_at: Utc::now(),
        };

        *self.user.write() = Some(user);
        *self.token.write() = Some(token.to_string());
    }
}

impl Default for AuthStore {
    fn default() -> Self {
        Self::new()
    }
}
