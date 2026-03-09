use crate::auth::AuthService;
use crate::cache::Cache;
use crate::config::Config;
use crate::domain::admin::AdminDomainService;
use crate::domain::auth::DefaultAuthDomainService;
use crate::domain::image::ImageDomainService;
use crate::domain::image::repository::{PostgresCategoryRepository, PostgresImageRepository};
use crate::runtime_settings::RuntimeSettingsService;
use crate::storage_backend::StorageManager;
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub redis: ConnectionManager,
    pub config: Config,
    pub auth: AuthService,
    pub auth_domain_service: Option<Arc<DefaultAuthDomainService>>,
    pub image_domain_service:
        Option<Arc<ImageDomainService<PostgresImageRepository, PostgresCategoryRepository>>>,
    pub admin_domain_service: Option<Arc<AdminDomainService>>,
    pub runtime_settings: Arc<RuntimeSettingsService>,
    pub storage_manager: Arc<StorageManager>,
    pub started_at: Instant,
}

impl AppState {
    pub async fn invalidate_user_image_cache(&self, user_id: Uuid) -> Result<(), anyhow::Error> {
        let mut redis = self.redis.clone();
        Cache::del_pattern(
            &mut redis,
            &crate::cache::ImageCache::images_invalidate(user_id),
        )
        .await
    }

    pub async fn invalidate_user_category_cache(&self, user_id: Uuid) -> Result<(), anyhow::Error> {
        let mut redis = self.redis.clone();
        Cache::del_pattern(
            &mut redis,
            &crate::cache::ImageCache::categories_invalidate(user_id),
        )
        .await
    }

    pub async fn invalidate_user_caches(&self, user_id: Uuid) -> Result<(), anyhow::Error> {
        self.invalidate_user_image_cache(user_id).await?;
        self.invalidate_user_category_cache(user_id).await?;
        Ok(())
    }
}
