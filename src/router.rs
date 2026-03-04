//! 路由配置模块
//! 负责配置应用的所有路由

use crate::routes::create_routes;
use crate::config::Config;
use crate::db::AppState;
use axum::{Router, response::Html, extract::Request};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use axum::http::{header, Method};

/// SPA fallback 处理器
async fn handle_spa(_req: Request) -> Html<String> {
    match tokio::fs::read_to_string("frontend/dist/index.html").await {
        Ok(content) => Html(content),
        Err(e) => {
            tracing::error!("Failed to read index.html: {}", e);
            Html("<h1>Frontend not found</h1><p>Please run 'npm run build' in frontend directory.</p>".to_string())
        }
    }
}

/// 创建静态文件服务路由
pub fn create_static_routes(config: &Config) -> Router {
    let images_serve_dir = ServeDir::new(&config.storage.path);
    let frontend_dist = ServeDir::new("frontend/dist");
    let assets_dir = ServeDir::new("frontend/dist/assets");

    Router::new()
        .nest_service("/images", images_serve_dir)
        .nest_service("/assets", assets_dir)
        .nest_service("/favicon.ico", frontend_dist.clone())
        .nest_service("/user.png", frontend_dist.clone())
        .fallback_service(frontend_dist)
        .fallback(handle_spa)
}

/// 创建CORS层
pub fn create_cors_layer(config: &Config) -> CorsLayer {
    if config.server.cors_origins == "*" {
        // 开发环境：允许所有来源
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
    } else {
        // 生产环境：使用配置的来源
        let origins: Vec<axum::http::HeaderValue> = config.server.cors_origins
            .split(',')
            .map(|s: &str| s.trim().parse().expect("Invalid CORS origin"))
            .collect();

        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
    }
}

/// 创建完整的应用路由
pub fn create_app_router(state: AppState, config: &Config) -> Router {
    let api_routes = Router::new()
        .merge(create_routes())
        .with_state(state);

    let static_routes = create_static_routes(config);
    let cors = create_cors_layer(config);

    api_routes
        .merge(static_routes)
        .layer(cors)
}

/// 创建带中间件的应用路由
pub fn create_app_with_middleware(
    state: AppState,
    config: &Config,
    max_upload_size: usize,
) -> Router {
    let router = create_app_router(state, config);

    router
        .layer(axum::extract::DefaultBodyLimit::max(max_upload_size))
}
