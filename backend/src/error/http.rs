use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

use super::app_error::AppError;

#[derive(serde::Serialize)]
struct ErrorResponseBody {
    error: String,
    code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

fn status_for(error: &AppError) -> StatusCode {
    match error {
        AppError::Unauthorized => StatusCode::UNAUTHORIZED,
        AppError::InvalidToken => StatusCode::UNAUTHORIZED,
        AppError::InvalidPassword => StatusCode::UNAUTHORIZED,
        AppError::HashError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        AppError::PasswordAlreadyUsed => StatusCode::BAD_REQUEST,
        AppError::UserNotFound => StatusCode::NOT_FOUND,
        AppError::UsernameExists => StatusCode::CONFLICT,
        AppError::MailServiceNotEnabled => StatusCode::SERVICE_UNAVAILABLE,
        AppError::ResetTokenExpired => StatusCode::BAD_REQUEST,
        AppError::ResetTokenInvalid => StatusCode::BAD_REQUEST,
        AppError::ImageNotFound => StatusCode::NOT_FOUND,
        AppError::InvalidImageFormat => StatusCode::BAD_REQUEST,
        AppError::Forbidden => StatusCode::FORBIDDEN,
        AppError::AdminRequired => StatusCode::FORBIDDEN,
        AppError::InvalidUsernameLength => StatusCode::BAD_REQUEST,
        AppError::InvalidPasswordLength => StatusCode::BAD_REQUEST,
        AppError::EmptyCategoryName => StatusCode::BAD_REQUEST,
        AppError::RateLimitExceeded(_) => StatusCode::TOO_MANY_REQUESTS,
        AppError::InvalidPagination => StatusCode::BAD_REQUEST,
        AppError::ValidationError(_) => StatusCode::BAD_REQUEST,
        AppError::StorageBackendMisconfigured(_) => StatusCode::SERVICE_UNAVAILABLE,
        AppError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        AppError::CacheError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        AppError::IoError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn is_dev_env() -> bool {
    let env = std::env::var("ENV")
        .or_else(|_| std::env::var("RUST_ENV"))
        .or_else(|_| std::env::var("NODE_ENV"))
        .unwrap_or_default()
        .to_lowercase();

    env == "development" || env.starts_with("dev")
}

fn message_for(error: &AppError, is_dev: bool) -> String {
    if is_dev {
        match error {
            AppError::DatabaseError(source) => format!("数据库错误: {}", source),
            AppError::CacheError(source) => format!("缓存错误: {}", source),
            AppError::IoError(source) => format!("文件操作失败: {}", source),
            AppError::Internal(source) => format!("内部错误: {}", source),
            AppError::MailServiceNotEnabled => "邮件服务未启用，请联系管理员配置".to_string(),
            AppError::StorageBackendMisconfigured(source) => {
                format!("存储后端配置无效: {}", source)
            }
            _ => error.to_string(),
        }
    } else {
        match error {
            AppError::DatabaseError(_) => "服务暂时不可用".to_string(),
            AppError::CacheError(_) => "服务暂时不可用".to_string(),
            AppError::IoError(_) => "文件操作失败".to_string(),
            AppError::Internal(_) => "内部服务器错误".to_string(),
            AppError::MailServiceNotEnabled => "邮件服务未启用，请联系管理员".to_string(),
            AppError::StorageBackendMisconfigured(_) => "存储服务当前不可用".to_string(),
            _ => error.to_string(),
        }
    }
}

fn details_for(error: &AppError, is_dev: bool) -> Option<String> {
    if !is_dev {
        return None;
    }

    match error {
        AppError::DatabaseError(source) => Some(format!("{:?}", source)),
        AppError::CacheError(source) => Some(format!("{:?}", source)),
        AppError::IoError(source) => Some(format!("{:?}", source)),
        AppError::Internal(source) => Some(format!("{:?}", source)),
        AppError::StorageBackendMisconfigured(source) => Some(source.clone()),
        _ => None,
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        error!("request failed: {:?}", self);

        let is_dev = is_dev_env();
        let body = ErrorResponseBody {
            error: message_for(&self, is_dev),
            code: self.code(),
            details: details_for(&self, is_dev),
        };

        (status_for(&self), Json(body)).into_response()
    }
}
