#![allow(unused_imports)]
//! 认证领域模块

pub mod claims;
pub mod domain_service;
pub mod repository;
pub mod service;

#[cfg(test)]
pub mod mock_repository;

// 导出领域层类型
pub use claims::Claims;
pub use domain_service::AuthDomainService;
pub use repository::{AuthRepository, PostgresAuthRepository};
pub use service::AuthService;

// 创建具体类型别名用于 AppState
pub type DefaultAuthDomainService = AuthDomainService<PostgresAuthRepository>;

// 重新导出关联的模型类型
pub use crate::models::User;
pub use crate::models::{LoginRequest, UpdateProfileRequest, UserResponse};
