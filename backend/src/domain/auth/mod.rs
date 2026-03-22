//! 认证领域模块

pub mod claims;
pub mod domain_service;
pub mod repository;
pub mod service;
pub mod state_repository;

#[cfg(test)]
pub mod mock_repository;

// 导出领域层类型
pub use claims::Claims;
use domain_service::AuthDomainService;
pub use repository::{DatabaseAuthRepository, PostgresAuthRepository};
pub use service::AuthService;

// 创建具体类型别名用于 AppState
pub type DefaultAuthDomainService = AuthDomainService<DatabaseAuthRepository>;
