use crate::config::Config;
use axum::{
    Router,
    http::{HeaderValue, header},
    routing,
};
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;

pub(super) fn create_frontend_routes(config: &Config) -> Router {
    let frontend_dist = ServeDir::new(&config.server.frontend_dir);
    let frontend_assets = ServeDir::new(format!("{}/assets", config.server.frontend_dir));
    let spa_fallback = routing::get_service(
        ServeDir::new(&config.server.frontend_dir).append_index_html_on_directories(true),
    )
    .layer(SetResponseHeaderLayer::overriding(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate, max-age=0"),
    ));

    Router::new()
        .nest_service("/assets", frontend_assets)
        .nest_service("/favicon.ico", frontend_dist)
        .fallback_service(spa_fallback)
}
