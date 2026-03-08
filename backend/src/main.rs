mod audit;
mod auth;
mod cache;
mod config;
mod db;
mod domain;
pub mod error;
mod file_queue;
mod handlers;
mod image_processor;
mod infrastructure;
mod middleware;
mod models;
mod router;
mod routes;
mod server;
mod tasks;
mod utils;

// handlers 子模块
pub use handlers::admin as admin_handlers;
pub use handlers::auth as auth_handlers;
pub use handlers::images as image_handlers;
pub use handlers::images_cursor;

use auth::AuthService;
use config::Config;
use domain::admin::AdminDomainService;
use domain::auth::{AuthDomainService, PostgresAuthRepository};
use domain::image::{ImageDomainService, PostgresCategoryRepository, PostgresImageRepository};
use image_processor::ImageProcessor;
use redis::Client;
use router::create_app_with_middleware;
use server::start_server;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::{Level, error, info};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

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

    error!("Configuration loaded (log level: {})", log_level);

    info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.url)
        .await?;

    info!("Initializing database schema...");
    db::init_schema(&pool).await?;

    // 创建管理员账户（仅首次）
    match db::create_admin_account(&pool).await {
        Ok(_) => {
            info!("Admin account initialized successfully");
        }
        Err(e) => {
            error!("Failed to initialize admin account: {}", e);
        }
    }

    // 打印管理员凭证
    db::log_admin_credentials();

    info!("Connecting to Redis...");
    let redis_client = Client::open(config.redis.url.clone())?;
    let redis_conn = redis_client.get_connection_manager().await?;

    let auth = AuthService::new(&config)
        .map_err(|e| anyhow::anyhow!("Failed to initialize auth service: {}", e))?;

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
    let file_save_queue = Arc::new(file_queue::FileSaveQueue::new(
        redis_conn.clone(),
        format!("{}image_save_queue", config.redis.key_prefix),
    ));
    info!(
        "File save task queue initialized (Redis backed: {}image_save_queue)",
        config.redis.key_prefix
    );

    // 初始化认证领域服务
    let auth_repository = PostgresAuthRepository::new(pool.clone());
    let auth_domain_service = Arc::new(AuthDomainService::new(auth_repository, auth.clone()));

    info!("Auth domain service initialized");

    // 初始化图片领域服务
    let image_repository = PostgresImageRepository::new(pool.clone());
    let category_repository = PostgresCategoryRepository::new(pool.clone());
    let image_domain_service = ImageDomainService::new(
        pool.clone(),
        Some(redis_conn.clone()),
        config.clone(),
        image_repository,
        category_repository,
        image_processor.clone(),
        file_save_queue.clone(),
    );
    let image_domain_service = Arc::new(image_domain_service);
    info!("Image domain service initialized");

    // 初始化管理领域服务
    let admin_domain_service =
        AdminDomainService::new(pool.clone(), Some(redis_conn.clone()), config.clone());
    let admin_domain_service = Arc::new(admin_domain_service);
    info!("Admin domain service initialized");

    let state = db::AppState {
        pool,
        redis: redis_conn,
        config: config.clone(),
        auth,
        auth_domain_service: Some(auth_domain_service),
        image_domain_service: Some(image_domain_service),
        admin_domain_service: Some(admin_domain_service),
        image_processor,
        file_save_queue,
        started_at: std::time::Instant::now(),
    };

    // 创建应用路由和中间件
    let app = create_app_with_middleware(state.clone(), &config, config.server.max_upload_size)
        .layer(TraceLayer::new_for_http());

    // 启动服务器
    let addr = config.addr();
    let listener = server::bind_listener(addr).await?;

    // 启动服务器
    start_server(listener, app).await?;

    Ok(())
}
