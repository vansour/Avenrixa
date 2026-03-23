//! 管理领域服务
//!
//! 封装管理相关的业务逻辑

mod backup;
mod health;
mod maintenance;
mod queries;
mod settings;

use crate::cache::CacheConnection;
use crate::config::Config;
use crate::db::DatabasePool;
use crate::observability::RuntimeObservability;
use crate::storage_backend::StorageManager;
use std::sync::Arc;

/// 管理领域服务
pub struct AdminDomainService {
    pub(super) database: DatabasePool,
    pub(super) cache: Option<CacheConnection>,
    pub(super) config: Config,
    pub(super) storage_manager: Arc<StorageManager>,
    pub(super) observability: Arc<RuntimeObservability>,
}

impl AdminDomainService {
    pub fn new(
        database: DatabasePool,
        cache: Option<CacheConnection>,
        config: Config,
        storage_manager: Arc<StorageManager>,
        observability: Arc<RuntimeObservability>,
    ) -> Self {
        Self {
            database,
            cache,
            config,
            storage_manager,
            observability,
        }
    }
}
