use super::cache::connect_cache;
use super::database::initialize_database;
use super::services::build_services;
use crate::background_tasks::BackgroundTaskManager;
use crate::config::Config;
use crate::db::{AppState, DatabasePool};
use crate::domain::auth::state_repository::DatabaseAuthStateRepository;
use crate::observability::RuntimeObservability;
use std::sync::Arc;

pub async fn build_app_state(config: Config) -> anyhow::Result<AppState> {
    let database = initialize_database(&config).await?;
    assemble_app_state(config, database).await
}

async fn assemble_app_state(config: Config, database: DatabasePool) -> anyhow::Result<AppState> {
    let cache_connections = connect_cache(&config).await;
    let observability = Arc::new(RuntimeObservability::new());
    let services = build_services(
        &database,
        &cache_connections,
        &config,
        observability.clone(),
    )
    .await?;

    Ok(AppState {
        auth_state_repository: DatabaseAuthStateRepository::from_database(&database),
        database,
        cache: cache_connections.app,
        config,
        auth: services.auth,
        auth_domain_service: services.auth_domain_service,
        image_domain_service: services.image_domain_service,
        admin_domain_service: services.admin_domain_service,
        runtime_settings: services.runtime_settings,
        storage_manager: services.storage_manager,
        observability,
        background_tasks: std::sync::Arc::new(BackgroundTaskManager::new()),
        installation_lock: std::sync::Arc::new(tokio::sync::Mutex::new(())),
        started_at: std::time::Instant::now(),
    })
}
