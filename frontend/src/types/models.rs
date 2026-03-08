use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// 图片项
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageItem {
    pub id: Uuid,
    pub user_id: Uuid,
    pub category_id: Option<Uuid>,
    pub filename: String,
    pub thumbnail: Option<String>,
    pub original_filename: Option<String>,
    pub size: i64,
    pub hash: String,
    pub format: String,
    pub views: i32,
    pub status: String,  // "active", "deleted"
    pub expires_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl ImageItem {
    pub fn url(&self) -> String {
        format!("/images/{}", self.id)
    }

    pub fn thumbnail_url(&self) -> Option<String> {
        self.thumbnail.as_ref().map(|_| format!("/thumbnails/{}", self.id))
    }

    pub fn original_filename(&self) -> String {
        self.original_filename.as_ref().cloned().unwrap_or_else(|| self.filename.clone())
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

    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < Utc::now()
        } else {
            false
        }
    }
}

/// 图片过滤器
#[derive(Debug, Clone)]
pub struct ImageFilters {
    pub search: Option<String>,
    pub category_id: Option<String>,
    pub sort_by: String,
    pub sort_order: String,
}

impl Default for ImageFilters {
    fn default() -> Self {
        Self {
            search: None,
            category_id: None,
            sort_by: "created_at".to_string(),
            sort_order: "desc".to_string(),
        }
    }
}

impl ImageFilters {
    pub fn new() -> Self {
        Self::default()
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
