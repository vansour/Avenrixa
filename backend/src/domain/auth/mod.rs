//! 认证领域模块

pub mod claims;
pub mod domain_service;
pub mod repository;
pub mod service;

#[cfg(test)]
pub mod mock_repository;

// 导出领域层类型
pub use claims::Claims;
use domain_service::AuthDomainService;
pub use repository::{
    DatabaseAuthRepository, MySqlAuthRepository, PostgresAuthRepository, SqliteAuthRepository,
};
pub use service::AuthService;
use uuid::Uuid;

// 创建具体类型别名用于 AppState
pub type DefaultAuthDomainService = AuthDomainService<DatabaseAuthRepository>;

pub fn user_token_version_key(user_id: Uuid) -> String {
    format!("user_token_version:{}", user_id)
}

pub fn auth_valid_after_key() -> &'static str {
    crate::sqlite_restore::auth_valid_after_key()
}
