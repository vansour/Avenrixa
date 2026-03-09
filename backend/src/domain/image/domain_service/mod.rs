//! 图片领域服务
//!
//! 封装图片相关的业务逻辑

mod categories;
mod common;
mod cursor;
mod delete_restore;
mod duplicate;
mod queries;
mod update;
mod upload;

#[cfg(test)]
mod tests;

use chrono::Utc;
use futures::stream::{self, StreamExt};
use redis::AsyncCommands;
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use super::repository::{CategoryRepository, ImageRepository};
use crate::audit::log_audit;
use crate::cache::{Cache, HashCache, ImageCache};
use crate::config::Config;
use crate::error::AppError;
use crate::file_queue::{FileSaveQueue, FileSaveTask};
use crate::image_processor::ImageProcessor;
use crate::models::{Category, Image, Paginated};
use crate::storage_backend::StorageManager;
use tracing::{info, warn};

/// 图片哈希检查结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct ImageInfo {
    pub id: Uuid,
    pub filename: String,
    pub user_id: Uuid,
}

const MAX_TAGS_PER_IMAGE: usize = 20;
const MAX_TAG_LENGTH: usize = 50;

pub struct ImageDomainServiceDependencies {
    pub pool: PgPool,
    pub redis: Option<redis::aio::ConnectionManager>,
    pub config: Config,
    pub image_processor: ImageProcessor,
    pub file_save_queue: Arc<FileSaveQueue>,
    pub storage_manager: Arc<StorageManager>,
}

impl ImageDomainServiceDependencies {
    pub fn new(
        pool: PgPool,
        redis: Option<redis::aio::ConnectionManager>,
        config: Config,
        image_processor: ImageProcessor,
        file_save_queue: Arc<FileSaveQueue>,
        storage_manager: Arc<StorageManager>,
    ) -> Self {
        Self {
            pool,
            redis,
            config,
            image_processor,
            file_save_queue,
            storage_manager,
        }
    }
}

/// 图片领域服务
pub struct ImageDomainService<I: ImageRepository, C: CategoryRepository> {
    pool: PgPool,
    redis: Option<redis::aio::ConnectionManager>,
    config: Config,
    image_repository: I,
    category_repository: C,
    image_processor: ImageProcessor,
    file_save_queue: Arc<FileSaveQueue>,
    storage_manager: Arc<StorageManager>,
}
