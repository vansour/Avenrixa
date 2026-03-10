use axum::{Router, routing};

use crate::bootstrap::BootstrapAppState;
use crate::config::Config;
use crate::handlers::bootstrap;

use super::frontend;

pub fn create_bootstrap_router(state: BootstrapAppState, config: &Config) -> Router {
    Router::new()
        .route("/health", routing::get(bootstrap::bootstrap_health_check))
        .route(
            "/api/v1/bootstrap/status",
            routing::get(bootstrap::get_bootstrap_status),
        )
        .route(
            "/api/v1/bootstrap/database-config",
            routing::put(bootstrap::update_bootstrap_database_config),
        )
        .merge(frontend::create_bootstrap_frontend_routes(config))
        .with_state(state)
}
