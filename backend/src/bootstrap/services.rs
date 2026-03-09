use super::redis::RedisConnections;
use crate::auth::AuthService;
use crate::config::Config;
use crate::domain::admin::AdminDomainService;
use crate::domain::auth::{DefaultAuthDomainService, PostgresAuthRepository};
use crate::domain::image::{
    ImageDomainService, ImageDomainServiceDependencies, PostgresCategoryRepository,
    PostgresImageRepository,
};
use crate::file_queue::FileSaveQueue;
use crate::image_processor::ImageProcessor;
use crate::runtime_settings::RuntimeSettingsService;
use crate::storage_backend::StorageManager;
use sqlx::PgPool;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use tracing::info;

pub type DefaultImageDomainService =
    ImageDomainService<PostgresImageRepository, PostgresCategoryRepository>;

pub struct ServiceBundle {
    pub auth: AuthService,
    pub auth_domain_service: Arc<DefaultAuthDomainService>,
    pub image_domain_service: Arc<DefaultImageDomainService>,
    pub admin_domain_service: Arc<AdminDomainService>,
    pub runtime_settings: Arc<RuntimeSettingsService>,
    pub storage_manager: Arc<StorageManager>,
}

pub async fn build_services(
    pool: &PgPool,
    redis_connections: &RedisConnections,
    config: &Config,
) -> anyhow::Result<ServiceBundle> {
    let auth = AuthService::new(config)
        .map_err(|error| anyhow::anyhow!("Failed to initialize auth service: {}", error))?;

    prepare_storage(config).await?;

    let image_processor = ImageProcessor::new(
        config.image.max_width,
        config.image.max_height,
        config.image.thumbnail_size,
        config.image.jpeg_quality,
    );
    let runtime_settings = Arc::new(RuntimeSettingsService::new(pool.clone(), config));
    let storage_manager = Arc::new(StorageManager::new(runtime_settings.clone()));
    storage_manager.ensure_local_storage_dir().await?;

    let file_save_queue = Arc::new(FileSaveQueue::new(
        redis_connections.queue.clone(),
        redis_connections.worker.clone(),
        format!("{}image_save_queue", config.redis.key_prefix),
    ));
    info!(
        "File save task queue initialized (Redis backed: {}image_save_queue)",
        config.redis.key_prefix
    );

    let auth_repository = PostgresAuthRepository::new(pool.clone());
    let auth_domain_service = Arc::new(DefaultAuthDomainService::new(auth_repository));
    info!("Auth domain service initialized");

    let image_repository = PostgresImageRepository::new(pool.clone());
    let category_repository = PostgresCategoryRepository::new(pool.clone());
    let image_dependencies = ImageDomainServiceDependencies::new(
        pool.clone(),
        Some(redis_connections.app.clone()),
        config.clone(),
        image_processor,
        file_save_queue,
        storage_manager.clone(),
    );
    let image_domain_service = Arc::new(DefaultImageDomainService::new(
        image_dependencies,
        image_repository,
        category_repository,
    ));
    info!("Image domain service initialized");

    let admin_domain_service = Arc::new(AdminDomainService::new(
        pool.clone(),
        Some(redis_connections.app.clone()),
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

async fn prepare_storage(config: &Config) -> anyhow::Result<()> {
    info!("Creating storage directories...");
    tokio::fs::create_dir_all(&config.storage.path).await?;

    let images_perms = PermissionsExt::from_mode(0o755);
    let _ = tokio::fs::set_permissions(&config.storage.path, images_perms).await;

    Ok(())
}
