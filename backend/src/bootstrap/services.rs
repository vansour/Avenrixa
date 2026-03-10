use super::redis::RedisConnections;
use crate::auth::AuthService;
use crate::config::Config;
use crate::db::DatabasePool;
use crate::domain::admin::AdminDomainService;
use crate::domain::auth::{
    DatabaseAuthRepository, DefaultAuthDomainService, PostgresAuthRepository, SqliteAuthRepository,
};
use crate::domain::image::{
    DatabaseImageRepository, DefaultImageDomainService, ImageDomainServiceDependencies,
    PostgresImageRepository, SqliteImageRepository,
};
use crate::image_processor::ImageProcessor;
use crate::runtime_settings::{RuntimeSettings, RuntimeSettingsService, StorageBackend};
use crate::storage_backend::StorageManager;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use tracing::info;

pub struct ServiceBundle {
    pub auth: AuthService,
    pub auth_domain_service: Option<Arc<DefaultAuthDomainService>>,
    pub image_domain_service: Option<Arc<DefaultImageDomainService>>,
    pub admin_domain_service: Option<Arc<AdminDomainService>>,
    pub runtime_settings: Arc<RuntimeSettingsService>,
    pub storage_manager: Arc<StorageManager>,
}

pub async fn build_services(
    database: &DatabasePool,
    redis_connections: &RedisConnections,
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
    prepare_storage(&active_runtime_settings).await?;
    let storage_manager = Arc::new(StorageManager::new(active_runtime_settings));

    let auth_repository = match database {
        DatabasePool::Postgres(pool) => {
            DatabaseAuthRepository::Postgres(PostgresAuthRepository::new(pool.clone()))
        }
        DatabasePool::Sqlite(pool) => {
            DatabaseAuthRepository::Sqlite(SqliteAuthRepository::new(pool.clone()))
        }
    };
    let auth_domain_service = Some(Arc::new(DefaultAuthDomainService::new(auth_repository)));
    info!("Auth domain service initialized");

    let image_repository = match database {
        DatabasePool::Postgres(pool) => {
            DatabaseImageRepository::Postgres(PostgresImageRepository::new(pool.clone()))
        }
        DatabasePool::Sqlite(pool) => {
            DatabaseImageRepository::Sqlite(SqliteImageRepository::new(pool.clone()))
        }
    };
    let image_dependencies = ImageDomainServiceDependencies::new(
        database.clone(),
        Some(redis_connections.app.clone()),
        config.clone(),
        image_processor,
        storage_manager.clone(),
    );
    let image_domain_service = Some(Arc::new(DefaultImageDomainService::new(
        image_dependencies,
        image_repository,
    )));
    info!("Image domain service initialized");

    let admin_domain_service = Some(Arc::new(AdminDomainService::new(
        database.clone(),
        Some(redis_connections.app.clone()),
        config.clone(),
        storage_manager.clone(),
    )));
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

async fn prepare_storage(settings: &RuntimeSettings) -> anyhow::Result<()> {
    if settings.storage_backend != StorageBackend::Local {
        return Ok(());
    }

    info!("Creating storage directories...");
    tokio::fs::create_dir_all(&settings.local_storage_path).await?;

    let images_perms = PermissionsExt::from_mode(0o755);
    let _ = tokio::fs::set_permissions(&settings.local_storage_path, images_perms).await;

    Ok(())
}
