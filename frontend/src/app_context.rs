use crate::services::{ApiClient, AuthService, ImageService};
use crate::store::auth::AuthStore;
use crate::store::images::ImageStore;
use crate::store::toast_store::ToastStore;
use dioxus::prelude::*;

/// 应用上下文
#[derive(Clone)]
pub struct AppContext {
    pub auth_store: AuthStore,
    pub image_store: ImageStore,
    pub api_client: ApiClient,
    pub auth_service: AuthService,
    pub image_service: ImageService,
    pub toast_store: ToastStore,
}

impl AppContext {
    /// 创建新的应用上下文
    pub fn new(base_url: String) -> Self {
        let auth_store = AuthStore::new();
        let image_store = ImageStore::new();
        let toast_store = ToastStore::new();
        let api_client = ApiClient::new(base_url);

        // 直接创建服务（使用 clone 避免移动）
        let auth_service = AuthService::new(api_client.clone(), auth_store.clone());
        let image_service = ImageService::new(api_client.clone(), image_store.clone());

        Self {
            auth_store,
            image_store,
            api_client,
            auth_service,
            image_service,
            toast_store,
        }
    }
}

/// AppContext Hook - 获取 AuthStore
pub fn use_auth_store() -> AuthStore {
    use_context()
}

/// AppContext Hook - 获取 ImageStore
pub fn use_image_store() -> ImageStore {
    use_context()
}

/// AppContext Hook - 获取 ApiClient
pub fn use_api_client() -> ApiClient {
    use_context()
}

/// AppContext Hook - 获取 AuthService
pub fn use_auth_service() -> AuthService {
    use_context()
}

/// AppContext Hook - 获取 ImageService
pub fn use_image_service() -> ImageService {
    use_context()
}

/// AppContext Hook - 获取 ToastStore
pub fn use_toast_store() -> ToastStore {
    use_context()
}
