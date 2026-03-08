//! 图片领域模块

pub mod domain_service;
pub mod repository;

#[cfg(test)]
pub mod mock_repository;

// 仅导出外部需要使用的类型
pub use domain_service::ImageDomainService;
pub use repository::{PostgresCategoryRepository, PostgresImageRepository};
