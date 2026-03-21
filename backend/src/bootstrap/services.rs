use super::cache::CacheConnections;
use crate::auth::AuthService;
use crate::config::Config;
use crate::db::DatabasePool;
use crate::domain::admin::AdminDomainService;
use crate::domain::auth::{
    DatabaseAuthRepository, DefaultAuthDomainService, PostgresAuthRepository,
};
use crate::domain::image::{
    DatabaseImageRepository, DefaultImageDomainService, ImageDomainServiceDependencies,
    PostgresImageRepository,
};
use crate::image_processor::ImageProcessor;
use crate::runtime_settings::RuntimeSettingsService;
use crate::storage_backend::StorageManager;
use std::sync::Arc;
use tracing::info;

pub struct ServiceBundle {
    pub auth: AuthService,
    pub auth_domain_service: Arc<DefaultAuthDomainService>,
    pub image_domain_service: Arc<DefaultImageDomainService>,
    pub admin_domain_service: Arc<AdminDomainService>,
    pub runtime_settings: Arc<RuntimeSettingsService>,
    pub storage_manager: Arc<StorageManager>,
}

pub async fn build_services(
    database: &DatabasePool,
    cache_connections: &CacheConnections,
    config: &Config,
) -> anyhow::Result<ServiceBundle> {
    let auth = AuthService::new(config)
        .map_err(|error| anyhow::anyhow!("Failed to initialize auth service: {}", error))?;

    let image_processor = ImageProcessor::new(
        config.image.max_width,
        config.image.max_height,
        config.image.jpeg_quality,
    );
    let runtime_settings = Arc::new(RuntimeSettingsService::new(database.clone(), config));
    let active_runtime_settings = runtime_settings.get_runtime_settings().await?;
    let storage_manager = Arc::new(StorageManager::new(active_runtime_settings.clone()));
    storage_manager
        .apply_runtime_settings(active_runtime_settings)
        .await?;

    let auth_repository = match database {
        DatabasePool::Postgres(pool) => {
            DatabaseAuthRepository::Postgres(PostgresAuthRepository::new(pool.clone()))
        }
    };
    let auth_domain_service = Arc::new(DefaultAuthDomainService::new(auth_repository));
    info!("Auth domain service initialized");

    let image_domain_service = match database {
        DatabasePool::Postgres(pool) => {
            let image_repository =
                DatabaseImageRepository::Postgres(PostgresImageRepository::new(pool.clone()));
            let image_dependencies = ImageDomainServiceDependencies::new(
                database.clone(),
                cache_connections.app.clone(),
                config.clone(),
                image_processor.clone(),
                storage_manager.clone(),
            );
            Arc::new(DefaultImageDomainService::new(
                image_dependencies,
                image_repository,
            ))
        }
    };
    info!("Image domain service initialized");

    let admin_domain_service = Arc::new(AdminDomainService::new(
        database.clone(),
        cache_connections.app.clone(),
        config.clone(),
        storage_manager.clone(),
    ));
    info!("Admin domain service initialized");

    Ok(ServiceBundle {
        auth,
        auth_domain_service,
        image_domain_service,
        admin_domain_service,
        runtime_settings,
        storage_manager,
    })
}
