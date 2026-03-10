use crate::services::api_client::ApiClient;
use crate::store::images::{ImageCollectionKind, ImageStore};
use crate::types::api::{
    DeleteRequest, Paginated, PaginationParams, RestoreRequest, SetExpiryRequest,
    UpdateImageRequest,
};
use crate::types::errors::Result;
use crate::types::models::ImageItem;

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
        let query_params = Self::build_query_params(&params);
        let url = if query_params.is_empty() {
            "/api/v1/images".to_string()
        } else {
            format!("/api/v1/images?{}", query_params)
        };

        let result = self
            .api_client
            .get_json::<Paginated<ImageItem>>(&url)
            .await?;
        self.image_store.replace_page(
            ImageCollectionKind::Active,
            result.data.clone(),
            result.page.max(1) as u32,
            params.page_size.unwrap_or(result.page_size).max(1) as u32,
            result.total.max(0) as u64,
            result.has_next,
        );
        Ok(result)
    }

    /// 上传图片
    pub async fn upload_image(
        &self,
        filename: String,
        content_type: Option<String>,
        bytes: Vec<u8>,
    ) -> Result<ImageItem> {
        self.api_client
            .post_multipart_file("/api/v1/upload", "file", filename, content_type, bytes)
            .await
    }

    /// 获取单张图片
    pub async fn get_image(&self, image_key: &str) -> Result<ImageItem> {
        let url = format!("/api/v1/images/{}", image_key);
        self.api_client.get_json(&url).await
    }

    /// 更新图片标签
    pub async fn update_image(&self, image_key: &str, req: UpdateImageRequest) -> Result<()> {
        let url = format!("/api/v1/images/{}", image_key);
        self.api_client.put_json_no_response(&url, &req).await
    }

    /// 设置过期时间
    pub async fn set_expiry(
        &self,
        image_key: &str,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<()> {
        let url = format!("/api/v1/images/{}/expiry", image_key);
        let req = SetExpiryRequest { expires_at };
        self.api_client.put_json_no_response(&url, &req).await
    }

    /// 批量删除图片
    pub async fn delete_images(&self, image_keys: Vec<String>, permanent: bool) -> Result<()> {
        let url = "/api/v1/images".to_string();
        let body = DeleteRequest {
            image_keys,
            permanent,
        };
        self.api_client.delete_json(&url, &body).await
    }

    /// 获取已删除图片
    pub async fn get_deleted_images(
        &self,
        params: PaginationParams,
    ) -> Result<Paginated<ImageItem>> {
        let query_params = Self::build_query_params(&params);
        let url = if query_params.is_empty() {
            "/api/v1/images/deleted".to_string()
        } else {
            format!("/api/v1/images/deleted?{}", query_params)
        };

        let result = self
            .api_client
            .get_json::<Paginated<ImageItem>>(&url)
            .await?;
        self.image_store.replace_page(
            ImageCollectionKind::Deleted,
            result.data.clone(),
            result.page.max(1) as u32,
            params.page_size.unwrap_or(result.page_size).max(1) as u32,
            result.total.max(0) as u64,
            result.has_next,
        );
        Ok(result)
    }

    /// 恢复图片
    pub async fn restore_images(&self, image_keys: Vec<String>) -> Result<()> {
        let url = "/api/v1/images/restore".to_string();
        let req = RestoreRequest { image_keys };
        self.api_client.post_json_no_response(&url, &req).await
    }

    /// 构建 URL 查询参数
    fn build_query_params(params: &PaginationParams) -> String {
        let mut query_parts = Vec::new();

        if let Some(page) = params.page {
            query_parts.push(format!("page={}", page));
        }
        if let Some(page_size) = params.page_size {
            query_parts.push(format!("page_size={}", page_size));
        }
        push_query_param(&mut query_parts, "tag", params.tag.as_deref());

        query_parts.join("&")
    }
}

fn push_query_param(parts: &mut Vec<String>, key: &str, value: Option<&str>) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    parts.push(format!("{}={}", key, urlencoding::encode(value)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_query_params_encodes_filter_options() {
        let params = PaginationParams {
            page: Some(2),
            page_size: Some(40),
            tag: Some("cover art".to_string()),
        };

        let query = ImageService::build_query_params(&params);

        assert!(query.contains("page=2"));
        assert!(query.contains("page_size=40"));
        assert!(query.contains("tag=cover%20art"));
    }

    #[test]
    fn build_query_params_omits_blank_values() {
        let params = PaginationParams {
            page: Some(1),
            page_size: Some(20),
            tag: Some("   ".to_string()),
        };

        let query = ImageService::build_query_params(&params);

        assert_eq!(query, "page=1&page_size=20");
    }
}
