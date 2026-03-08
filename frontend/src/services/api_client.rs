use reqwest::Client;
use crate::store::auth::AuthStore;
use crate::types::errors::{Result, AppError};

/// API 客户端（简化版）
pub struct ApiClient {
    client: Client,
    base_url: String,
    auth_store: AuthStore,
}

impl ApiClient {
    pub fn new(base_url: String, auth_store: AuthStore) -> Self {
        Self {
            client: Client::new(),
            base_url,
            auth_store,
        }
    }

    /// 构建完整 URL
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}
