use axum::http::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
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
    #[error("用户不存在")]
    UserNotFound,
    #[error("用户名已存在")]
    UsernameExists,
    #[error("邮件服务未启用")]
    MailServiceNotEnabled,
    #[error("重置令牌已过期")]
    ResetTokenExpired,
    #[error("重置令牌无效")]
    ResetTokenInvalid,
    #[error("图片不存在")]
    ImageNotFound,
    #[error("无效的图片格式")]
    InvalidImageFormat,
    #[error("权限不足")]
    Forbidden,
    #[error("需要管理员权限")]
    AdminRequired,
    #[error("用户名长度必须在3-50 字符之间")]
    InvalidUsernameLength,
    #[error("密码长度至少为 6 个字符")]
    InvalidPasswordLength,
    #[error("分类名称不能为空")]
    EmptyCategoryName,
    #[error("无效的分页参数")]
    InvalidPagination,
    #[error("{0}")]
    ValidationError(String),
    #[error("存储后端配置无效: {0}")]
    StorageBackendMisconfigured(String),
    #[error("数据库操作失败")]
    DatabaseError(#[from] sqlx::Error),
    #[error("缓存操作失败")]
    CacheError(#[from] redis::RedisError),
    #[error("文件操作失败")]
    IoError(#[from] std::io::Error),
    #[error("内部服务器错误")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
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
            Self::Forbidden => "FORBIDDEN",
            Self::AdminRequired => "ADMIN_REQUIRED",
            Self::InvalidUsernameLength => "INVALID_USERNAME_LENGTH",
            Self::InvalidPasswordLength => "INVALID_PASSWORD_LENGTH",
            Self::EmptyCategoryName => "EMPTY_CATEGORY_NAME",
            Self::RateLimitExceeded(_) => "RATE_LIMIT_EXCEEDED",
            Self::InvalidPagination => "INVALID_PAGINATION",
            Self::ValidationError(_) => "VALIDATION_ERROR",
            Self::StorageBackendMisconfigured(_) => "STORAGE_BACKEND_MISCONFIGURED",
            Self::DatabaseError(_) => "DATABASE_ERROR",
            Self::CacheError(_) => "CACHE_ERROR",
            Self::IoError(_) => "IO_ERROR",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }

    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            Self::Unauthorized
                | Self::InvalidToken
                | Self::InvalidPassword
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
                | Self::ValidationError(_)
        )
    }

    pub fn is_server_error(&self) -> bool {
        !self.is_client_error()
    }
}

impl From<StatusCode> for AppError {
    fn from(status: StatusCode) -> Self {
        match status {
            StatusCode::UNAUTHORIZED => AppError::Unauthorized,
            StatusCode::FORBIDDEN => AppError::Forbidden,
            StatusCode::NOT_FOUND => AppError::ImageNotFound,
            StatusCode::BAD_REQUEST => AppError::ValidationError("请求参数错误".to_string()),
            StatusCode::CONFLICT => AppError::UsernameExists,
            _ => AppError::Internal(anyhow::anyhow!("HTTP error: {}", status)),
        }
    }
}
