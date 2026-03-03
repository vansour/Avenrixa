use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub username: Option<String>,
    pub current_password: String,
    pub new_password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            role: user.role,
            created_at: user.created_at,
        }
    }
}

impl axum::response::IntoResponse for UserResponse {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        axum::Json(self).into_response()
    }
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Category {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Image {
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
    pub status: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    #[serde(skip)] // 用于窗口函数返回的总数，不序列化到API响应
    #[sqlx(default, skip)] // 数据库查询时忽略，但允许存在
    pub total_count: Option<i64>,
}

impl Image {
    pub fn url(&self) -> String {
        format!("/images/{}", self.id)
    }

    pub fn thumbnail_url(&self) -> Option<String> {
        self.thumbnail.as_ref().map(|_| format!("/thumbnails/{}", self.id))
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

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub search: Option<String>,
    pub category_id: Option<Uuid>,
    pub tag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Paginated<T> {
    pub data: Vec<T>,
    pub page: i32,
    pub page_size: i32,
    pub total: i64,
    pub has_next: bool,
}

#[derive(Debug, Deserialize)]
pub struct RestoreRequest {
    pub image_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteRequest {
    pub image_ids: Vec<Uuid>,
    pub permanent: bool,
}

#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    pub filename: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryRequest {
    pub category_id: Option<Uuid>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct SetExpiryRequest {
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ApproveRequest {
    pub image_ids: Vec<Uuid>,
    pub approved: bool,
}

#[derive(Debug, Deserialize)]
pub struct DuplicateRequest {
    pub image_id: Uuid,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct EditImageRequest {
    pub crop: Option<CropParams>,
    pub rotate: Option<i32>,
    pub filters: Option<FilterParams>,
    pub convert_format: Option<String>,
    pub watermark: Option<WatermarkParams>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CropParams {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FilterParams {
    pub brightness: Option<i32>,
    pub contrast: Option<i32>,
    pub saturation: Option<i32>,
    pub grayscale: Option<bool>,
    pub sepia: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WatermarkParams {
    pub text: Option<String>,
    pub position: Option<String>,
    pub opacity: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EditImageResponse {
    pub id: Uuid,
    pub edited_url: String,
    pub thumbnail_url: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<Uuid>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub data: Vec<AuditLog>,
    pub page: i32,
    pub page_size: i32,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
pub struct UserUpdateRequest {
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingRequest {
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub database: ComponentStatus,
    pub redis: ComponentStatus,
    pub storage: ComponentStatus,
    /// 应用版本号（从环境变量 APP_VERSION 读取）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// 运行时间（秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<u64>,
    /// 系统指标
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<HealthMetrics>,
}

#[derive(Debug, Serialize)]
pub struct HealthMetrics {
    pub images_count: i64,
    pub users_count: i64,
    pub storage_used_mb: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ComponentStatus {
    pub status: String,
    pub message: Option<String>,
}

impl ComponentStatus {
    pub fn healthy() -> Self {
        Self {
            status: "healthy".to_string(),
            message: None,
        }
    }

    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            status: "unhealthy".to_string(),
            message: Some(message.into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SystemStats {
    pub total_users: i64,
    pub total_images: i64,
    pub total_storage: i64,
    pub total_views: i64,
    pub images_last_24h: i64,
    pub images_last_7d: i64,
}

impl axum::response::IntoResponse for SystemStats {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        axum::Json(self).into_response()
    }
}

#[derive(Debug, Serialize)]
pub struct BackupResponse {
    pub filename: String,
    pub created_at: DateTime<Utc>,
}

/// 统一的 API 响应格式 (预留，用于未来 API 版本升级)
#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
    pub message: Option<String>,
}

#[allow(dead_code)]
impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data,
            message: None,
        }
    }

    pub fn success_with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            success: true,
            data,
            message: Some(message.into()),
        }
    }
}

#[allow(dead_code)]
impl axum::response::IntoResponse for ApiResponse<()> {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        axum::Json(self).into_response()
    }
}

#[allow(dead_code)]
impl axum::response::IntoResponse for ApiResponse<Image> {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        axum::Json(self).into_response()
    }
}

#[allow(dead_code)]
impl axum::response::IntoResponse for ApiResponse<EditImageResponse> {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        axum::Json(self).into_response()
    }
}
