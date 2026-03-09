//! 路由配置模块
//! 负责配置应用的所有路由

use crate::config::Config;
use crate::db::AppState;
use crate::routes::create_routes;
use axum::{
    Router,
    http::{HeaderValue, header},
    routing,
};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;

/// 创建完整的应用路由
pub fn create_app_router(state: AppState, config: &Config) -> Router {
    let state_cloned = state.clone();
    let thumb_state = state.clone();
    let image_state = state.clone();
    let api_routes = create_routes().with_state(state);
    let mut governor_conf = GovernorConfigBuilder::default();
    governor_conf
        .per_second(config.server.rate_limit_per_second as u64)
        .burst_size(config.server.rate_limit_burst);
    let governor_conf = governor_conf
        .finish()
        .expect("Invalid rate limit configuration");

    // 仅对 API 路由做限流，避免图片/静态资源加载被 429 误伤
    let api_routes_v1 = Router::new()
        .nest("/api/v1", api_routes)
        .layer(GovernorLayer::new(governor_conf));

    let health_route = Router::new()
        .route("/health", routing::get(crate::admin_handlers::health_check))
        .with_state(state_cloned);
    let image_route = Router::new()
        .route(
            "/images/{filename}",
            routing::get(crate::routes::serve_image),
        )
        .with_state(image_state);
    let thumbnail_route = Router::new()
        .route(
            "/thumbnails/{image_key}",
            routing::get(crate::routes::serve_thumbnail),
        )
        .with_state(thumb_state);

    let frontend_dist = ServeDir::new(&config.server.frontend_dir);
    let frontend_assets = ServeDir::new(format!("{}/assets", config.server.frontend_dir));
    let spa_fallback = routing::get_service(
        ServeDir::new(&config.server.frontend_dir).append_index_html_on_directories(true),
    )
    .layer(SetResponseHeaderLayer::overriding(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate, max-age=0"),
    ));

    // 路由顺序很重要：先 API 路由，再静态文件，最后 SPA fallback
    // ServeDir 会自动处理 SPA fallback（找不到文件时返回 index.html）
    Router::new()
        .merge(health_route)
        .merge(image_route)
        .merge(thumbnail_route)
        .merge(api_routes_v1)
        .nest_service("/assets", frontend_assets)
        .nest_service("/favicon.ico", frontend_dist)
        .fallback_service(spa_fallback)
}

/// 创建带中间件的应用路由
pub fn create_app_with_middleware(
    state: AppState,
    config: &Config,
    max_upload_size: usize,
) -> Router {
    let router = create_app_router(state, config);
    router.layer(axum::extract::DefaultBodyLimit::max(max_upload_size))
}
