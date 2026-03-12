//! 图片领域服务
//!
//! 封装图片相关的业务逻辑

mod common;
mod delete_restore;
mod queries;
mod update;
mod upload;

#[cfg(test)]
mod tests;

use chrono::Utc;
use futures::stream::{self, StreamExt};
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

use super::repository::ImageRepository;
use crate::audit::log_audit_db;
use crate::cache::{Cache, CacheConnection, HashCache, ImageCache};
use crate::config::Config;
use crate::db::DatabasePool;
use crate::error::AppError;
use crate::image_processor::ImageProcessor;
use crate::models::{Image, Paginated};
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
    pub database: DatabasePool,
    pub cache: Option<CacheConnection>,
    pub config: Config,
    pub image_processor: ImageProcessor,
    pub storage_manager: Arc<StorageManager>,
}

impl ImageDomainServiceDependencies {
    pub fn new(
        database: DatabasePool,
        cache: Option<CacheConnection>,
        config: Config,
        image_processor: ImageProcessor,
        storage_manager: Arc<StorageManager>,
    ) -> Self {
        Self {
            database,
            cache,
            config,
            image_processor,
            storage_manager,
        }
    }
}

/// 图片领域服务
pub struct ImageDomainService<I: ImageRepository> {
    database: DatabasePool,
    cache: Option<CacheConnection>,
    config: Config,
    image_repository: I,
    image_processor: ImageProcessor,
    storage_manager: Arc<StorageManager>,
}
