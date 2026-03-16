use crate::services::api_client::ApiClient;
use crate::store::images::{ImageCollectionKind, ImageStore};
use crate::types::api::{
    DeleteRequest, ImageResponse, Paginated, PaginationParams, SetExpiryRequest,
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
            .get_json::<Paginated<ImageResponse>>(&url)
            .await?;
        let result = map_paginated_images(result);
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
        let image = self
            .api_client
            .post_multipart_file::<ImageResponse>(
                "/api/v1/upload",
                "file",
                filename,
                content_type,
                bytes,
            )
            .await?;
        Ok(ImageItem::from(image))
    }

    /// 获取单张图片
    pub async fn get_image(&self, image_key: &str) -> Result<ImageItem> {
        let url = format!("/api/v1/images/{}", image_key);
        let image: ImageResponse = self.api_client.get_json(&url).await?;
        Ok(ImageItem::from(image))
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
    pub async fn delete_images(&self, image_keys: Vec<String>) -> Result<()> {
        let url = "/api/v1/images".to_string();
        let body = DeleteRequest { image_keys };
        self.api_client.delete_json(&url, &body).await
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
        query_parts.join("&")
    }
}

fn map_paginated_images(result: Paginated<ImageResponse>) -> Paginated<ImageItem> {
    Paginated {
        data: result.data.into_iter().map(ImageItem::from).collect(),
        page: result.page,
        page_size: result.page_size,
        total: result.total,
        has_next: result.has_next,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_query_params_keeps_declared_pagination_order() {
        let params = PaginationParams {
            page: Some(2),
            page_size: Some(40),
        };

        let query = ImageService::build_query_params(&params);

        assert_eq!(query, "page=2&page_size=40");
    }

    #[test]
    fn build_query_params_omits_absent_values() {
        let params = PaginationParams {
            page: None,
            page_size: Some(20),
        };

        let query = ImageService::build_query_params(&params);

        assert_eq!(query, "page_size=20");
    }

    #[test]
    fn map_paginated_images_converts_api_items_to_view_items() {
        let result = map_paginated_images(Paginated {
            data: vec![ImageResponse {
                image_key: "img_1".to_string(),
                filename: "demo.png".to_string(),
                size: 128,
                format: "png".to_string(),
                views: 1,
                status: crate::types::models::ImageStatus::Active,
                expires_at: None,
                created_at: chrono::Utc::now(),
            }],
            page: 2,
            page_size: 20,
            total: 41,
            has_next: true,
        });

        assert_eq!(result.data.len(), 1);
        assert_eq!(result.data[0].filename, "demo.png");
        assert_eq!(result.page, 2);
        assert!(result.has_next);
    }
}
