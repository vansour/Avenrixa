mod audit;
mod auth;
mod cache;
mod config;
mod db;
mod email;
pub mod error;
mod handlers;
mod image_processor;
mod middleware;
mod models;
mod routes;
mod utils;
mod validator_utils;

// handlers 子模块
pub use handlers::auth as auth_handlers;
pub use handlers::images as image_handlers;
pub use handlers::categories as category_handlers;
pub use handlers::admin as admin_handlers;

use axum::{
    extract::DefaultBodyLimit,
    http::{header, Method},
    http::HeaderValue,
    Router,
};
use auth::AuthService;
use config::Config;
use image_processor::ImageProcessor;
use redis::Client;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
    services::ServeDir,
};
use tracing::{error, info, warn, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use routes::create_routes;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 从环境变量 RUST_LOG 读取日志级别，默认 INFO
    let log_level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(Level::INFO);

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive(log_level.into()))
        .init();

    let config = Config::from_env();
    info!("Configuration loaded (log level: {})", log_level);

    info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.url)
        .await?;

    info!("Initializing database schema...");
    db::init_schema(&pool).await?;

    // 检查并创建默认管理员账号
    match db::create_default_admin(&pool).await {
        Ok(_) => {
            info!("Default admin account initialized successfully");
        }
        Err(e) => {
            warn!("Failed to initialize default admin account: {}", e);
        }
    }

    // 每次启动时输出管理员账号信息到日志
    match db::log_admin_credentials(&pool).await {
        Ok(_) => {}
        Err(e) => {
            warn!("Failed to log admin credentials: {}", e);
        }
    }

    info!("Connecting to Redis...");
    let redis_client = Client::open(config.redis.url.clone())?;
    let redis_conn = redis_client.get_connection_manager().await?;

    let auth = AuthService::new(&config).map_err(|e| anyhow::anyhow!("Failed to initialize auth service: {}", e))?;

    info!("Creating storage directories...");
    tokio::fs::create_dir_all(&config.storage.path).await?;
    tokio::fs::create_dir_all(&config.storage.thumbnail_path).await?;

    let image_processor = ImageProcessor::new(
        config.image.max_width,
        config.image.max_height,
        config.image.thumbnail_size,
        config.image.jpeg_quality,
    );

    let state = db::AppState {
        pool,
        redis: redis_conn,
        config: config.clone(),
        auth,
        image_processor,
        started_at: std::time::Instant::now(),
    };

    let cors = if config.server.cors_origins == "*" {
        // 开发环境：允许所有来源
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
    } else {
        // 生产环境：使用配置的来源
        let origins: Vec<HeaderValue> = config.server.cors_origins
            .split(',')
            .map(|s| s.trim().parse().expect("Invalid CORS origin"))
            .collect();

        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
    };

    let api_routes = Router::new()
        .merge(create_routes())
        .with_state(state.clone());

    let static_assets = Router::new()
        .nest_service("/images", ServeDir::new(&config.storage.path))
        .nest_service("/thumbnails", ServeDir::new(&config.storage.thumbnail_path))
        .fallback_service(ServeDir::new("./frontend/dist").append_index_html_on_directories(true));

    // 配置限流中间件（ TODO: 待完善 tower_governor API）
    // let governor_config = GovernorConfig {
    //     interval: std::time::Duration::from_secs(60),
    //     burst_size: NonZeroUsize::new(config.rate_limit.burst_size as usize).unwrap(),
    //     quanta: NonZeroUsize::new(config.rate_limit.requests_per_minute as usize).unwrap(),
    // };
    // let governor_layer = GovernorLayer {
    //     config: std::sync::Arc::new(governor_config),
    // };

    let app = Router::new()
        .merge(api_routes)
        .merge(static_assets)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        // .layer(governor_layer) // 限流层（待完善）
        .layer(DefaultBodyLimit::max(config.server.max_upload_size));

    let addr: SocketAddr = config.addr();
    let listener = TcpListener::bind(addr).await?;

    let cleanup_pool = state.pool.clone();
    let retention_days = state.config.cleanup.deleted_images_retention_days;
    let storage_path = state.config.storage.path.clone();
    let thumbnail_path = state.config.storage.thumbnail_path.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(86400));
        loop {
            interval.tick().await;
            info!("Running cleanup task...");

            let days_ago = chrono::Utc::now() - chrono::Duration::days(retention_days);
            let result = sqlx::query_as::<_, (uuid::Uuid, String)>(
                "SELECT id, filename FROM images WHERE deleted_at < $1"
            )
            .bind(days_ago)
            .fetch_all(&cleanup_pool)
            .await;

            let images: Vec<(uuid::Uuid, String)> = match result {
                Ok(r) => r,
                Err(e) => {
                    error!("Failed to query cleanup images: {}", e);
                    continue;
                }
            };

            let mut removed_count = 0;
            for (id, filename) in &images {
                let file_storage_path = format!("{}/{}", storage_path, filename);
                let file_thumbnail_path = format!("{}/{}.jpg", thumbnail_path, id);

                let file_removed = tokio::fs::remove_file(&file_storage_path).await.is_ok();
                let thumb_removed = tokio::fs::remove_file(&file_thumbnail_path).await.is_ok();

                if file_removed || thumb_removed {
                    let _ = sqlx::query("DELETE FROM images WHERE id = $1")
                        .bind(id)
                        .execute(&cleanup_pool)
                        .await;
                    removed_count += 1;
                }
            }
            info!("Cleanup complete: {} images removed", removed_count);
        }
    });

    let expiry_pool = state.pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            let now = chrono::Utc::now();

            let result = sqlx::query(
                "UPDATE images SET deleted_at = $1 WHERE expires_at < $1 AND deleted_at IS NULL"
            )
            .bind(now)
            .execute(&expiry_pool)
            .await;

            if let Ok(r) = result
                && r.rows_affected() > 0 {
                    info!("Expired images moved to trash: {}", r.rows_affected());
                }
        }
    });

    info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
