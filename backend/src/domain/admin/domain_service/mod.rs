//! 管理领域服务
//!
//! 封装管理相关的业务逻辑

mod backup;
mod health;
mod maintenance;
mod queries;
mod restore;
mod settings;

use crate::cache::CacheConnection;
use crate::config::Config;
use crate::db::DatabasePool;
use crate::storage_backend::StorageManager;

/// 管理领域服务
pub struct AdminDomainService {
    pub(super) database: DatabasePool,
    pub(super) cache: Option<CacheConnection>,
    pub(super) config: Config,
    pub(super) storage_manager: std::sync::Arc<StorageManager>,
}

impl AdminDomainService {
    pub fn new(
        database: DatabasePool,
        cache: Option<CacheConnection>,
        config: Config,
        storage_manager: std::sync::Arc<StorageManager>,
    ) -> Self {
        Self {
            database,
            cache,
            config,
            storage_manager,
        }
    }
}
