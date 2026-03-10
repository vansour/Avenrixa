mod api;
mod bootstrap;
mod frontend;

use crate::bootstrap::BootstrapAppState;
use crate::config::Config;
use crate::db::AppState;
use axum::Router;

pub fn create_app_router(state: AppState, config: &Config) -> Router {
    Router::new()
        .merge(api::create_root_routes(state.clone()))
        .merge(api::create_api_v1_router(state.clone(), config))
        .merge(frontend::create_frontend_routes(state.clone(), config))
}

pub fn create_app_with_middleware(
    state: AppState,
    config: &Config,
    max_upload_size: usize,
) -> Router {
    create_app_router(state, config).layer(axum::extract::DefaultBodyLimit::max(max_upload_size))
}

pub fn create_bootstrap_app(
    state: BootstrapAppState,
    config: &Config,
    max_upload_size: usize,
) -> Router {
    bootstrap::create_bootstrap_router(state, config)
        .layer(axum::extract::DefaultBodyLimit::max(max_upload_size))
}
