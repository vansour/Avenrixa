use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 登录请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 用户响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
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
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            page_size: Some(20),
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
    pub next_cursor: Option<String>,
}

/// 批量删除请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRequest {
    pub image_keys: Vec<String>,
    pub permanent: bool,
}

/// 恢复请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreRequest {
    pub image_keys: Vec<String>,
}

/// 设置过期请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetExpiryRequest {
    pub expires_at: Option<DateTime<Utc>>,
}

/// 更新分类请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCategoryRequest {
    pub category_id: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// 修改密码请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    pub username: Option<String>,
    pub current_password: String,
    pub new_password: Option<String>,
}

/// 管理员设置配置（结构化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSettingsConfig {
    pub site_name: String,
    pub storage_backend: String,
    pub local_storage_path: String,
    pub s3_endpoint: Option<String>,
    pub s3_region: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_prefix: Option<String>,
    pub s3_access_key: Option<String>,
    pub s3_secret_key_set: bool,
    pub s3_force_path_style: bool,
    pub restart_required: bool,
}

/// 更新管理员设置请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAdminSettingsConfigRequest {
    pub site_name: String,
    pub storage_backend: String,
    pub local_storage_path: String,
    pub s3_endpoint: Option<String>,
    pub s3_region: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_prefix: Option<String>,
    pub s3_access_key: Option<String>,
    pub s3_secret_key: Option<String>,
    pub s3_force_path_style: Option<bool>,
}
