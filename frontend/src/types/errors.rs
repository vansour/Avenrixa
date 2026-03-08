use reqwest::StatusCode;
use thiserror::Error;

/// 应用错误类型
#[derive(Debug, Clone, Error)]
pub enum AppError {
    #[error("网络错误: {0}")]
    Network(String),

    #[error("未授权")]
    Unauthorized,

    #[error("未找到")]
    NotFound,

    #[error("禁止访问")]
    Forbidden,

    #[error("服务器错误: {0}")]
    Server(String),

    #[error("验证错误: {0}")]
    Validation(String),

    #[error("请求错误: {0}")]
    Request(String),
}

/// 结果类型别名
pub type Result<T> = std::result::Result<T, AppError>;

impl AppError {
    /// 判断是否需要重定向到登录页
    pub fn should_redirect_login(&self) -> bool {
        matches!(self, AppError::Unauthorized | AppError::Forbidden)
    }

    /// 从 reqwest 错误转换
    pub fn from_reqwest(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            AppError::Network("请求超时".to_string())
        } else if e.status() == Some(StatusCode::UNAUTHORIZED) {
            AppError::Unauthorized
        } else if e.status() == Some(StatusCode::FORBIDDEN) {
            AppError::Forbidden
        } else if e.status() == Some(StatusCode::NOT_FOUND) {
            AppError::NotFound
        } else {
            // reqwest 0.13 没有 is_connect()，统一作为网络错误处理
            AppError::Network("网络请求失败".to_string())
        }
    }
}

// 从 reqwest::Error 转换
impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::from_reqwest(e)
    }
}

// 从 serde_json::Error 转换
impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Server(e.to_string())
    }
}
