mod audit;
mod auth;
mod cache;
mod config;
mod db;
mod email;
mod file_queue;
pub mod error;
mod handlers;
mod image_processor;
mod middleware;
mod models;
mod routes;
mod router;
mod server;
mod tasks;
mod utils;

// handlers 子模块
pub use handlers::auth as auth_handlers;
pub use handlers::images as image_handlers;
pub use handlers::images_cursor as images_cursor;
pub use handlers::categories as category_handlers;
pub use handlers::admin as admin_handlers;

use auth::AuthService;
use config::Config;
use image_processor::ImageProcessor;
use redis::Client;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use router::create_app_with_middleware;
use server::{spawn_cleanup_tasks, start_server};

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

    // 验证配置
    if let Err(e) = config.validate() {
        error!("Configuration validation failed: {}", e);
        return Err(e.into());
    }

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

    // 设置目录权限为 755，确保 web 服务器可以读取
    use std::os::unix::fs::PermissionsExt;
    let images_perms = PermissionsExt::from_mode(0o755);
    let thumbs_perms = PermissionsExt::from_mode(0o755);
    let _ = tokio::fs::set_permissions(&config.storage.path, images_perms).await;
    let _ = tokio::fs::set_permissions(&config.storage.thumbnail_path, thumbs_perms).await;

    let image_processor = ImageProcessor::new(
        config.image.max_width,
        config.image.max_height,
        config.image.thumbnail_size,
        config.image.jpeg_quality,
    );

    // 初始化文件保存任务队列
    let file_save_queue = Arc::new(file_queue::FileSaveQueue::new());
    info!("File save task queue initialized");

    let state = db::AppState {
        pool,
        redis: redis_conn,
        config: config.clone(),
        auth,
        image_processor,
        file_save_queue,
        started_at: std::time::Instant::now(),
    };

    // 创建应用路由和中间件
    let app = create_app_with_middleware(state.clone(), &config, config.server.max_upload_size)
        .layer(TraceLayer::new_for_http());

    // 启动清理任务
    spawn_cleanup_tasks(&state);

    // 启动服务器
    let addr = config.addr();
    let listener = server::bind_listener(addr).await?;

    // 启动服务器
    start_server(listener, app).await?;

    Ok(())
}
