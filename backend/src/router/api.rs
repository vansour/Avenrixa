use crate::config::Config;
use crate::db::AppState;
use crate::routes::create_routes;
use axum::{Router, routing};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

pub(super) fn create_root_routes(state: AppState) -> Router {
    Router::new()
        .route("/health", routing::get(crate::admin_handlers::health_check))
        .route(
            "/images/{filename}",
            routing::get(crate::routes::serve_image),
        )
        .route(
            "/thumbnails/{image_key}",
            routing::get(crate::routes::serve_thumbnail),
        )
        .with_state(state)
}

pub(super) fn create_api_v1_router(state: AppState, config: &Config) -> Router {
    let api_routes = create_routes().with_state(state);
    let mut governor_conf = GovernorConfigBuilder::default();
    governor_conf
        .per_second(config.server.rate_limit_per_second as u64)
        .burst_size(config.server.rate_limit_burst);
    let governor_conf = governor_conf
        .finish()
        .expect("Invalid rate limit configuration");

    Router::new()
        .nest("/api/v1", api_routes)
        .layer(GovernorLayer::new(governor_conf))
}
