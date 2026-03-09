use chrono::{DateTime, Utc};
use serde::Serialize;

use super::image::{EditImageResponse, Image};

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
