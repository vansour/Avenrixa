use crate::config::Config;
use crate::db::AppState;
use crate::handlers::admin;
use crate::routes::{api, media};
use axum::{Router, routing};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

pub(super) fn create_root_routes(state: AppState) -> Router {
    Router::new()
        .route("/health", routing::get(admin::health_check))
        .route("/images/{filename}", routing::get(media::serve_image))
        .route(
            "/thumbnails/{image_key}",
            routing::get(media::serve_thumbnail),
        )
        .with_state(state)
}

pub(super) fn create_api_v1_router(state: AppState, config: &Config) -> Router {
    let boot_routes = api::create_boot_public_routes().with_state(state.clone());
    let throttled_api_routes = api::create_throttled_api_routes().with_state(state);
    let mut governor_conf = GovernorConfigBuilder::default();
    governor_conf
        .per_second(config.server.rate_limit_per_second as u64)
        .burst_size(config.server.rate_limit_burst);
    let governor_conf = governor_conf
        .finish()
        .expect("Invalid rate limit configuration");

    Router::new().nest("/api/v1", boot_routes).nest(
        "/api/v1",
        throttled_api_routes.layer(GovernorLayer::new(governor_conf)),
    )
}
