//! 管理领域服务
//!
//! 封装管理相关的业务逻辑

mod backup;
mod health;
mod maintenance;
mod queries;
mod settings;

use sqlx::PgPool;

use crate::config::Config;
use crate::storage_backend::StorageManager;

use redis::aio::ConnectionManager;

/// 管理领域服务
pub struct AdminDomainService {
    pub(super) pool: PgPool,
    pub(super) redis: Option<ConnectionManager>,
    pub(super) config: Config,
    pub(super) storage_manager: std::sync::Arc<StorageManager>,
}

impl AdminDomainService {
    pub fn new(
        pool: PgPool,
        redis: Option<ConnectionManager>,
        config: Config,
        storage_manager: std::sync::Arc<StorageManager>,
    ) -> Self {
        Self {
            pool,
            redis,
            config,
            storage_manager,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}
