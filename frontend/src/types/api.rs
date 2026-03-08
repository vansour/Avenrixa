use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 登录请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 用户响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

/// 分页参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
    pub sort_by: String,
    pub sort_order: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// 游标分页参数（用于替代 OFFSET 分页）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<(DateTime<Utc>, String)>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            page_size: Some(20),
            sort_by: "created_at".to_string(),
            sort_order: "DESC".to_string(),
            search: None,
            category_id: None,
            tag: None,
            cursor: None,
        }
    }
}

/// 分页响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paginated<T> {
    pub data: Vec<T>,
    pub page: i32,
    pub page_size: i32,
    pub total: i64,
    pub has_next: bool,
}

/// Cursor-based 分页响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPaginated<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<(DateTime<Utc>, String)>,
}

/// 批量删除请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRequest {
    pub image_ids: Vec<Uuid>,
    pub permanent: bool,
}

/// 恢复请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreRequest {
    pub image_ids: Vec<Uuid>,
}

/// 重命名请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameRequest {
    pub filename: String,
}

/// 设置过期请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetExpiryRequest {
    pub expires_at: Option<DateTime<Utc>>,
}

/// 复制请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateRequest {
    pub image_id: Uuid,
}

/// 更新分类请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCategoryRequest {
    pub category_id: Option<Uuid>,
    pub tags: Option<Vec<String>>,
}

/// 修改密码请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    pub username: Option<String>,
    pub current_password: String,
    pub new_password: Option<String>,
}
