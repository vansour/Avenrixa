use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

/// 统一的应用错误类型
#[derive(Debug, Error)]
pub enum AppError {
    /// 认证相关错误
    #[error("未授权访问")]
    Unauthorized,

    #[error("无效的认证令牌")]
    InvalidToken,

    #[error("密码不正确")]
    InvalidPassword,

    #[error("密码哈希错误")]
    HashError(#[from] bcrypt::BcryptError),

    #[error("密码已被使用，请选择新密码")]
    PasswordAlreadyUsed,

    #[error("请求过于频繁，请稍后重试: {0}")]
    RateLimitExceeded(String),

    /// 用户相关错误
    #[error("用户不存在")]
    UserNotFound,

    #[error("用户名已存在")]
    UsernameExists,

    /// 邮件相关错误
    #[error("邮件服务未启用")]
    MailServiceNotEnabled,

    #[error("重置令牌已过期")]
    ResetTokenExpired,

    #[error("重置令牌无效")]
    ResetTokenInvalid,

    /// 图片相关错误
    #[error("图片不存在")]
    ImageNotFound,

    #[error("无效的图片格式")]
    InvalidImageFormat,

    #[error("图片处理失败")]
    ImageProcessingFailed { source: anyhow::Error },

    /// 权限相关错误
    #[error("权限不足")]
    Forbidden,

    #[error("需要管理员权限")]
    AdminRequired,

    /// 输入验证错误
    #[error("用户名长度必须在3-50 字符之间")]
    InvalidUsernameLength,

    #[error("密码长度至少为 6 个字符")]
    InvalidPasswordLength,

    #[error("分类名称不能为空")]
    EmptyCategoryName,

    #[error("无效的分页参数")]
    InvalidPagination,

    /// 数据库相关错误
    #[error("数据库操作失败")]
    DatabaseError(#[from] sqlx::Error),

    /// Redis 相关错误
    #[error("缓存操作失败")]
    CacheError(#[from] redis::RedisError),

    /// IO 相关错误
    #[error("文件操作失败")]
    IoError(#[from] std::io::Error),

    /// 其他内部错误
    #[error("内部服务器错误")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    /// 获取错误码（用于程序化错误处理）
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unauthorized => "UNAUTHORIZED",
            Self::InvalidToken => "INVALID_TOKEN",
            Self::InvalidPassword => "INVALID_PASSWORD",
            Self::HashError(_) => "HASH_ERROR",
            Self::PasswordAlreadyUsed => "PASSWORD_ALREADY_USED",
            Self::UserNotFound => "USER_NOT_FOUND",
            Self::UsernameExists => "USERNAME_EXISTS",
            Self::MailServiceNotEnabled => "MAIL_SERVICE_NOT_ENABLED",
            Self::ResetTokenExpired => "RESET_TOKEN_EXPIRED",
            Self::ResetTokenInvalid => "RESET_TOKEN_INVALID",
            Self::ImageNotFound => "IMAGE_NOT_FOUND",
            Self::InvalidImageFormat => "INVALID_IMAGE_FORMAT",
            Self::ImageProcessingFailed { .. } => "IMAGE_PROCESSING_FAILED",
            Self::Forbidden => "FORBIDDEN",
            Self::AdminRequired => "ADMIN_REQUIRED",
            Self::InvalidUsernameLength => "INVALID_USERNAME_LENGTH",
            Self::InvalidPasswordLength => "INVALID_PASSWORD_LENGTH",
            Self::EmptyCategoryName => "EMPTY_CATEGORY_NAME",
            Self::RateLimitExceeded(_) => "RATE_LIMIT_EXCEEDED",
            Self::InvalidPagination => "INVALID_PAGINATION",
            Self::DatabaseError(_) => "DATABASE_ERROR",
            Self::CacheError(_) => "CACHE_ERROR",
            Self::IoError(_) => "IO_ERROR",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }

    /// 判断是否为客户端错误（4xx）
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            Self::Unauthorized
                | Self::InvalidToken
                | Self::InvalidPassword
                | Self::HashError(_)
                | Self::PasswordAlreadyUsed
                | Self::UserNotFound
                | Self::UsernameExists
                | Self::MailServiceNotEnabled
                | Self::ResetTokenExpired
                | Self::ResetTokenInvalid
                | Self::ImageNotFound
                | Self::InvalidImageFormat
                | Self::Forbidden
                | Self::AdminRequired
                | Self::InvalidUsernameLength
                | Self::InvalidPasswordLength
                | Self::EmptyCategoryName
                | Self::RateLimitExceeded(_)
                | Self::InvalidPagination
        )
    }

    /// 判断是否为服务器错误（5xx）
    pub fn is_server_error(&self) -> bool {
        !self.is_client_error()
    }
}

/// 为 AppError 实现 IntoResponse
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
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
            AppError::ImageProcessingFailed { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::AdminRequired => StatusCode::FORBIDDEN,
            AppError::InvalidUsernameLength => StatusCode::BAD_REQUEST,
            AppError::InvalidPasswordLength => StatusCode::BAD_REQUEST,
            AppError::EmptyCategoryName => StatusCode::BAD_REQUEST,
            AppError::RateLimitExceeded(_) => StatusCode::TOO_MANY_REQUESTS,
            AppError::InvalidPagination => StatusCode::BAD_REQUEST,
            AppError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::CacheError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::IoError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        // 开发环境返回详细错误，生产环境只返回通用消息
        let env = std::env::var("ENV")
            .or_else(|_| std::env::var("RUST_ENV"))
            .or_else(|_| std::env::var("NODE_ENV"))
            .unwrap_or_default()
            .to_lowercase();

        let is_dev = env == "development" || env.starts_with("dev");
        let message = if is_dev {
            // 对于内部错误，返回具体错误信息
            match &self {
                AppError::ImageProcessingFailed { source } => {
                    format!("图片处理失败: {}", source)
                }
                AppError::DatabaseError(source) => {
                    format!("数据库错误: {}", source)
                }
                AppError::CacheError(source) => {
                    format!("缓存错误: {}", source)
                }
                AppError::IoError(source) => {
                    format!("文件操作失败: {}", source)
                }
                AppError::Internal(source) => {
                    format!("内部错误: {}", source)
                }
                AppError::MailServiceNotEnabled => "邮件服务未启用，请联系管理员配置".to_string(),
                _ => self.to_string(),
            }
        } else {
            // 生产环境使用通用消息，避免信息泄露
            match &self {
                AppError::ImageProcessingFailed { .. } => "图片处理失败，请稍后重试".to_string(),
                AppError::DatabaseError(_) => "服务暂时不可用".to_string(),
                AppError::CacheError(_) => "服务暂时不可用".to_string(),
                AppError::IoError(_) => "文件操作失败".to_string(),
                AppError::Internal(_) => "内部服务器错误".to_string(),
                AppError::MailServiceNotEnabled => "邮件服务未启用，请联系管理员".to_string(),
                _ => self.to_string(),
            }
        };

        #[derive(serde::Serialize)]
        struct ErrorResponse {
            error: String,
            code: &'static str,
            #[serde(skip_serializing_if = "Option::is_none")]
            details: Option<String>,
        }

        let mut response = ErrorResponse {
            error: message.clone(),
            code: self.code(),
            details: None,
        };

        // 开发环境添加额外详情
        if is_dev {
            match &self {
                AppError::ImageProcessingFailed { source } => {
                    response.details = Some(format!("{:?}", source));
                }
                AppError::DatabaseError(source) => {
                    response.details = Some(format!("{:?}", source));
                }
                AppError::CacheError(source) => {
                    response.details = Some(format!("{:?}", source));
                }
                AppError::IoError(source) => {
                    response.details = Some(format!("{:?}", source));
                }
                AppError::Internal(source) => {
                    response.details = Some(format!("{:?}", source));
                }
                _ => {}
            }
        }

        (status, Json(response)).into_response()
    }
}

/// From<StatusCode> 实现，方便将 StatusCode 转换为 AppError
impl From<StatusCode> for AppError {
    fn from(status: StatusCode) -> Self {
        match status {
            StatusCode::UNAUTHORIZED => AppError::Unauthorized,
            StatusCode::FORBIDDEN => AppError::Forbidden,
            StatusCode::NOT_FOUND => AppError::ImageNotFound,
            StatusCode::BAD_REQUEST => AppError::InvalidPagination,
            StatusCode::CONFLICT => AppError::UsernameExists,
            _ => AppError::Internal(anyhow::anyhow!("HTTP error: {}", status)),
        }
    }
}
