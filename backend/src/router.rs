//! 路由配置模块
//! 负责配置应用的所有路由

use crate::config::Config;
use crate::db::AppState;
use crate::routes::create_routes;
use axum::http::{Method, header};
use axum::{Router, routing};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};

/// 创建CORS层
pub fn create_cors_layer(config: &Config) -> CorsLayer {
    if config.server.cors_origins == "*" {
        // 开发环境：允许所有来源
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
    } else {
        // 生产环境：使用配置的来源
        let origins: Vec<axum::http::HeaderValue> = config
            .server
            .cors_origins
            .split(',')
            .map(|s: &str| s.trim().parse().expect("Invalid CORS origin"))
            .collect();

        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
    }
}

/// 创建完整的应用路由
pub fn create_app_router(state: AppState, config: &Config) -> Router {
    let state_cloned = state.clone();
    let api_routes = create_routes().with_state(state);
    let api_routes_v1 = Router::new().nest("/api/v1", api_routes);

    let health_route = Router::new()
        .route("/health", routing::get(crate::admin_handlers::health_check))
        .with_state(state_cloned);

    let images_serve_dir = ServeDir::new(format!("{}/images", config.storage.path));
    let frontend_dist = ServeDir::new("frontend-remix/build");
    let frontend_assets = ServeDir::new("frontend-remix/build/assets");

    let cors = create_cors_layer(config);

    // 路由顺序很重要：先 API 路由，再静态文件，最后 SPA fallback
    // ServeDir 会自动处理 SPA fallback（找不到文件时返回 index.html）
    Router::new()
        .merge(health_route)
        .merge(api_routes_v1)
        .nest_service("/images", images_serve_dir)
        .nest_service("/assets", frontend_assets)
        .nest_service("/favicon.ico", frontend_dist)
        .fallback_service(ServeDir::new("frontend-remix/build").append_index_html_on_directories(true))
        .layer(cors)
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
