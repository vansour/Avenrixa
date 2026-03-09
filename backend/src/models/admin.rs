use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ApproveRequest {
    pub image_ids: Vec<Uuid>,
    pub approved: bool,
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
pub struct UpdateSettingRequest {
    pub value: String,
}

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

#[derive(Debug, Clone, Deserialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<u64>,
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
