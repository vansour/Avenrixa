use crate::services::{ApiClient, AuthService, ImageService};
use crate::store::auth::AuthStore;
use crate::store::images::ImageStore;
use dioxus::prelude::*;
use parking_lot::RwLock;
use std::sync::Arc;

/// 应用上下文
#[derive(Clone)]
pub struct AppContext {
    pub auth_store: AuthStore,
    pub image_store: ImageStore,
    pub api_client: ApiClient,
    pub auth_service: Arc<RwLock<Option<AuthService>>>,
    pub image_service: Arc<RwLock<Option<ImageService>>>,
}

impl AppContext {
    /// 创建新的应用上下文
    pub fn new(base_url: String) -> Self {
        let auth_store = AuthStore::new();
        let image_store = ImageStore::new();
        let api_client = ApiClient::new(base_url, auth_store.clone());

        // 创建服务（延迟初始化，避免循环依赖）
        let auth_service = Arc::new(RwLock::new(None));
        let image_service = Arc::new(RwLock::new(None));

        Self {
            auth_store,
            image_store,
            api_client,
            auth_service,
            image_service,
        }
    }

    /// 获取或初始化认证服务
    pub fn get_auth_service(&self) -> AuthService {
        if self.auth_service.read().is_none() {
            let mut guard = self.auth_service.write();
            if guard.is_none() {
                *guard = Some(AuthService::new(
                    self.api_client.clone(),
                    self.auth_store.clone(),
                ));
            }
            guard.clone().unwrap()
        } else {
            self.auth_service.read().clone().unwrap()
        }
    }

    /// 获取或初始化图片服务
    pub fn get_image_service(&self) -> ImageService {
        if self.image_service.read().is_none() {
            let mut guard = self.image_service.write();
            if guard.is_none() {
                *guard = Some(ImageService::new(
                    self.api_client.clone(),
                    self.image_store.clone(),
                ));
            }
            guard.clone().unwrap()
        } else {
            self.image_service.read().clone().unwrap()
        }
    }
}

/// AppContext Hook
pub fn use_app_context() -> Signal<AppContext> {
    use_context::<Signal<AppContext>>()
}
