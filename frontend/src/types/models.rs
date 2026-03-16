use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
pub use shared_types::image::{ImageResponse, ImageStatus};

/// 图片项
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageItem {
    pub image_key: String,
    pub filename: String,
    pub size: i64,
    pub format: String,
    pub views: i32,
    pub status: ImageStatus,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl ImageItem {
    pub fn url(&self) -> String {
        format!("/images/{}", self.filename)
    }

    pub fn thumbnail_url(&self) -> String {
        format!("/thumbnails/{}.webp", self.image_key)
    }

    pub fn display_name(&self) -> String {
        self.filename.clone()
    }

    pub fn created_at_label(&self) -> String {
        self.created_at.format("%Y-%m-%d %H:%M").to_string()
    }

    pub fn size_formatted(&self) -> String {
        const KB: i64 = 1024;
        const MB: i64 = KB * 1024;
        const GB: i64 = MB * 1024;

        if self.size >= GB {
            format!("{:.2} GB", self.size as f64 / GB as f64)
        } else if self.size >= MB {
            format!("{:.2} MB", self.size as f64 / MB as f64)
        } else if self.size >= KB {
            format!("{:.2} KB", self.size as f64 / KB as f64)
        } else {
            format!("{} B", self.size)
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < Utc::now()
        } else {
            false
        }
    }
}

impl From<ImageResponse> for ImageItem {
    fn from(value: ImageResponse) -> Self {
        Self {
            image_key: value.image_key,
            filename: value.filename,
            size: value.size,
            format: value.format,
            views: value.views,
            status: value.status,
            expires_at: value.expires_at,
            created_at: value.created_at,
        }
    }
}

/// 分页响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

#[cfg(test)]
mod tests {
    use super::{ImageItem, ImageResponse, ImageStatus};
    use chrono::Utc;

    #[test]
    fn image_status_unknown_values_fall_back_safely() {
        assert_eq!(ImageStatus::parse("active"), ImageStatus::Active);
        assert_eq!(ImageStatus::parse("DELETED"), ImageStatus::Deleted);
        assert_eq!(ImageStatus::parse("archived"), ImageStatus::Unknown);
    }

    #[test]
    fn image_item_maps_from_api_response() {
        let now = Utc::now();
        let item = ImageItem::from(ImageResponse {
            image_key: "abc123".to_string(),
            filename: "demo.png".to_string(),
            size: 1024,
            format: "png".to_string(),
            views: 3,
            status: ImageStatus::Active,
            expires_at: None,
            created_at: now,
        });

        assert_eq!(item.image_key, "abc123");
        assert_eq!(item.filename, "demo.png");
        assert_eq!(item.status, ImageStatus::Active);
        assert_eq!(item.created_at, now);
    }
}
