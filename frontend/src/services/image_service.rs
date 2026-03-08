use crate::services::api_client::ApiClient;
use crate::store::images::ImageStore;
use crate::types::models::ImageItem;
use crate::types::errors::Result;

/// 图片服务
pub struct ImageService {
    api_client: ApiClient,
    image_store: ImageStore,
}

impl ImageService {
    pub fn new(api_client: ApiClient, image_store: ImageStore) -> Self {
        Self { api_client, image_store }
    }

    /// 获取图片列表
    pub async fn get_images(&self) -> Result<Vec<ImageItem>> {
        // TODO: 集成实际 API 调用
        let _url = self.api_client.url("/api/v1/images");
        self.image_store.set_loading(false);
        Ok(vec![])
    }
}
