#![allow(unused_imports)]
//! 认证领域模块

pub mod claims;
pub mod service;
pub mod repository;
pub mod domain_service;

#[cfg(test)]
pub mod mock_repository;

// 导出领域层类型
pub use claims::Claims;
pub use service::AuthService;
pub use repository::{AuthRepository, PostgresAuthRepository};
pub use domain_service::AuthDomainService;

// 创建具体类型别名用于 AppState
pub type DefaultAuthDomainService = AuthDomainService<PostgresAuthRepository>;

// 重新导出关联的模型类型
pub use crate::models::{LoginRequest, UpdateProfileRequest, UserResponse};
pub use crate::models::User;
