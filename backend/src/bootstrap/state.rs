use super::cache::connect_cache;
use super::database::initialize_database;
use super::services::build_services;
use crate::config::Config;
use crate::db::AppState;
use crate::domain::auth::state_repository::DatabaseAuthStateRepository;

pub async fn build_app_state(config: Config) -> anyhow::Result<AppState> {
    let database = initialize_database(&config).await?;
    let cache_connections = connect_cache(&config).await;
    let services = build_services(&database, &cache_connections, &config).await?;

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
        installation_lock: std::sync::Arc::new(tokio::sync::Mutex::new(())),
        started_at: std::time::Instant::now(),
    })
}
