use super::DatabasePool;
use crate::auth::AuthService;
use crate::cache::Cache;
use crate::config::Config;
use crate::config::DatabaseKind;
use crate::domain::admin::AdminDomainService;
use crate::domain::auth::DefaultAuthDomainService;
use crate::domain::image::DefaultImageDomainService;
use crate::runtime_settings::RuntimeSettingsService;
use crate::storage_backend::StorageManager;
use redis::aio::ConnectionManager;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub database: DatabasePool,
    pub redis: ConnectionManager,
    pub config: Config,
    pub auth: AuthService,
    pub auth_domain_service: Option<Arc<DefaultAuthDomainService>>,
    pub image_domain_service: Option<Arc<DefaultImageDomainService>>,
    pub admin_domain_service: Option<Arc<AdminDomainService>>,
    pub runtime_settings: Arc<RuntimeSettingsService>,
    pub storage_manager: Arc<StorageManager>,
    pub started_at: Instant,
}

impl AppState {
    pub fn database_kind(&self) -> DatabaseKind {
        self.database.kind()
    }

    pub fn postgres_pool(&self) -> anyhow::Result<&sqlx::PgPool> {
        self.database.postgres()
    }

    pub fn postgres_pool_owned(&self) -> anyhow::Result<sqlx::PgPool> {
        Ok(self.postgres_pool()?.clone())
    }

    pub async fn invalidate_user_image_cache(&self, user_id: Uuid) -> Result<(), anyhow::Error> {
        let mut redis = self.redis.clone();
        Cache::del_pattern(
            &mut redis,
            &crate::cache::ImageCache::images_invalidate(user_id),
        )
        .await
    }
}
