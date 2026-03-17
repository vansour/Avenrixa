use super::DatabasePool;
use crate::auth::AuthService;
use crate::cache::{Cache, CacheConnection};
use crate::config::Config;
use crate::config::DatabaseKind;
use crate::domain::admin::AdminDomainService;
use crate::domain::auth::DefaultAuthDomainService;
use crate::domain::auth::state_repository::DatabaseAuthStateRepository;
use crate::domain::image::DefaultImageDomainService;
use crate::runtime_settings::RuntimeSettingsService;
use crate::storage_backend::StorageManager;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub database: DatabasePool,
    pub cache: Option<CacheConnection>,
    pub auth_state_repository: DatabaseAuthStateRepository,
    pub config: Config,
    pub auth: AuthService,
    pub auth_domain_service: Arc<DefaultAuthDomainService>,
    pub image_domain_service: Arc<DefaultImageDomainService>,
    pub admin_domain_service: Arc<AdminDomainService>,
    pub runtime_settings: Arc<RuntimeSettingsService>,
    pub storage_manager: Arc<StorageManager>,
    pub installation_lock: Arc<Mutex<()>>,
    pub started_at: Instant,
}

impl AppState {
    pub fn database_kind(&self) -> DatabaseKind {
        self.database.kind()
    }

    pub fn postgres_pool(&self) -> anyhow::Result<&sqlx::PgPool> {
        self.database.postgres()
    }

    pub async fn invalidate_user_image_cache(&self, user_id: Uuid) -> Result<(), anyhow::Error> {
        let Some(mut cache) = self.cache.clone() else {
            return Ok(());
        };
        Cache::del_pattern(
            &mut cache,
            &crate::cache::ImageCache::images_invalidate(user_id),
        )
        .await
    }
}
