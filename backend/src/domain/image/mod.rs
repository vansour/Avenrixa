//! 图片领域模块

pub mod domain_service;
pub mod repository;

#[cfg(test)]
pub mod mock_repository;

pub use domain_service::{ImageDomainService, ImageDomainServiceDependencies};
pub use repository::{
    DatabaseImageRepository, MySqlImageRepository, PostgresImageRepository, SqliteImageRepository,
};

pub type DefaultImageDomainService = ImageDomainService<DatabaseImageRepository>;
