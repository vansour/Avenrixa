use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 图片项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageItem {
    pub id: String,
    pub filename: String,
    pub original_filename: Option<String>,
    pub size: i64,
    pub format: String,
    pub created_at: DateTime<Utc>,
    pub thumbnail_url: Option<String>,
    pub url: String,
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
