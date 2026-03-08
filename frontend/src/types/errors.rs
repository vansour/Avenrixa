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
}

/// 结果类型别名
pub type Result<T> = std::result::Result<T, AppError>;

impl AppError {
    /// 判断是否需要重定向到登录页
    pub fn should_redirect_login(&self) -> bool {
        matches!(self, AppError::Unauthorized | AppError::Forbidden)
    }
}

// 由于 reqwest::Error 的 From 实现需要访问 reqwest 的内部状态，
// 我们在 api_client 中手动处理错误转换
// 这里的 From 实现主要用于演示目的，实际中会由 API 客户端处理
