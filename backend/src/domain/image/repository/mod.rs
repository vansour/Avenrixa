//! 图片数据访问 trait
//!
//! 定义图片相关的数据访问接口

mod category_repository;
mod image_commands;
mod image_impl;
mod image_queries;
mod sql;
mod traits;

use sqlx::PgPool;

pub use traits::{CategoryRepository, ImageRepository};

/// PostgreSQL 图片仓库实现
pub struct PostgresImageRepository {
    pub(super) pool: PgPool,
}

impl PostgresImageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// PostgreSQL 分类仓库实现
pub struct PostgresCategoryRepository {
    pub(super) pool: PgPool,
}

impl PostgresCategoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
