use crate::services::api_client::ApiClient;
use crate::store::images::ImageStore;
use crate::types::api::{
    CursorPaginated, DeleteRequest, DuplicateRequest, Paginated, PaginationParams, RenameRequest,
    RestoreRequest, SetExpiryRequest, UpdateCategoryRequest,
};
use crate::types::errors::Result;
use crate::types::models::ImageItem;
use uuid::Uuid;

/// 图片服务
#[derive(Clone)]
pub struct ImageService {
    api_client: ApiClient,
    image_store: ImageStore,
}

impl ImageService {
    pub fn new(api_client: ApiClient, image_store: ImageStore) -> Self {
        Self {
            api_client,
            image_store,
        }
    }

    /// 获取图片列表（传统分页）
    pub async fn get_images(&self, params: PaginationParams) -> Result<Paginated<ImageItem>> {
        // 构建 URL 查询参数
        let query_params = self.build_query_params(&params);
        let url = if query_params.is_empty() {
            "/api/v1/images".to_string()
        } else {
            format!("/api/v1/images?{}", query_params)
        };

        let result = self
            .api_client
            .get_json::<Paginated<ImageItem>>(&url)
            .await?;
        self.image_store.set_images(result.data.clone());
        self.image_store.set_loading(false);
        Ok(result)
    }

    /// 获取图片列表（游标分页）
    pub async fn get_images_cursor(
        &self,
        params: PaginationParams,
    ) -> Result<CursorPaginated<ImageItem>> {
        let query_params = self.build_query_params(&params);
        let url = if query_params.is_empty() {
            "/api/v1/images/cursor".to_string()
        } else {
            format!("/api/v1/images/cursor?{}", query_params)
        };

        let result = self
            .api_client
            .get_json::<CursorPaginated<ImageItem>>(&url)
            .await?;
        self.image_store.set_images(result.data.clone());
        self.image_store.set_loading(false);
        Ok(result)
    }

    /// 获取单张图片
    pub async fn get_image(&self, id: Uuid) -> Result<ImageItem> {
        let url = format!("/api/v1/images/{}", id);
        self.api_client.get_json(&url).await
    }

    /// 更新图片（分类/标签）
    pub async fn update_image(&self, id: Uuid, req: UpdateCategoryRequest) -> Result<()> {
        let url = format!("/api/v1/images/{}", id);
        self.api_client.put_json_response(&url, &req).await
    }

    /// 重命名图片
    pub async fn rename_image(&self, id: Uuid, filename: String) -> Result<()> {
        let url = format!("/api/v1/images/{}/rename", id);
        let req = RenameRequest { filename };
        self.api_client.put_json_response(&url, &req).await
    }

    /// 设置过期时间
    pub async fn set_expiry(
        &self,
        id: Uuid,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<()> {
        let url = format!("/api/v1/images/{}/expiry", id);
        let req = SetExpiryRequest { expires_at };
        self.api_client.put_json_response(&url, &req).await
    }

    /// 复制图片
    pub async fn duplicate_image(&self, id: Uuid) -> Result<ImageItem> {
        let url = format!("/api/v1/images/{}/duplicate", id);
        let req = DuplicateRequest { image_id: id };
        self.api_client.post_json_response(&url, &req).await
    }

    /// 批量删除图片
    pub async fn delete_images(&self, image_ids: Vec<Uuid>, permanent: bool) -> Result<()> {
        let url = "/api/v1/images".to_string();
        let _body = serde_json::to_string(&DeleteRequest {
            image_ids,
            permanent,
        })
        .map_err(|e| crate::types::errors::AppError::Server(format!("JSON序列化失败: {}", e)))?;
        self.api_client.delete(&url).await
    }

    /// 获取已删除图片
    pub async fn get_deleted_images(
        &self,
        params: PaginationParams,
    ) -> Result<Paginated<ImageItem>> {
        let query_params = self.build_query_params(&params);
        let url = if query_params.is_empty() {
            "/api/v1/images/deleted".to_string()
        } else {
            format!("/api/v1/images/deleted?{}", query_params)
        };

        self.api_client.get_json(&url).await
    }

    /// 恢复图片
    pub async fn restore_images(&self, image_ids: Vec<Uuid>) -> Result<()> {
        let url = "/api/v1/images/restore".to_string();
        let req = RestoreRequest { image_ids };
        self.api_client.post_json_response(&url, &req).await
    }

    /// 构建 URL 查询参数
    fn build_query_params(&self, params: &PaginationParams) -> String {
        let mut query_parts = Vec::new();

        if let Some(page) = params.page {
            query_parts.push(format!("page={}", page));
        }
        if let Some(page_size) = params.page_size {
            query_parts.push(format!("page_size={}", page_size));
        }
        query_parts.push(format!("sort_by={}", params.sort_by));
        query_parts.push(format!("sort_order={}", params.sort_order));

        if let Some(ref search) = params.search {
            query_parts.push(format!("search={}", urlencoding::encode(search)));
        }
        if let Some(ref category_id) = params.category_id {
            query_parts.push(format!("category_id={}", category_id));
        }
        if let Some(ref tag) = params.tag {
            query_parts.push(format!("tag={}", urlencoding::encode(tag)));
        }
        if let Some(ref cursor) = params.cursor {
            // Cursor 是 (DateTime, String) 的元组
            let cursor_str = format!("{},{}", cursor.0.timestamp(), cursor.1);
            query_parts.push(format!("cursor={}", urlencoding::encode(&cursor_str)));
        }

        query_parts.join("&")
    }
}
