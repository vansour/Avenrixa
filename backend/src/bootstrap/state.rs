use super::database::initialize_database;
use super::redis::connect_redis;
use super::services::build_services;
use crate::config::Config;
use crate::db::AppState;

pub async fn build_app_state(config: Config) -> anyhow::Result<AppState> {
    let pool = initialize_database(&config).await?;
    let redis_connections = connect_redis(&config).await?;
    let services = build_services(&pool, &redis_connections, &config).await?;

    Ok(AppState {
        pool,
        redis: redis_connections.app,
        config,
        auth: services.auth,
        auth_domain_service: Some(services.auth_domain_service),
        image_domain_service: Some(services.image_domain_service),
        admin_domain_service: Some(services.admin_domain_service),
        runtime_settings: services.runtime_settings,
        storage_manager: services.storage_manager,
        started_at: std::time::Instant::now(),
    })
}
