mod audit;
mod auth;
mod bootstrap;
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
mod runtime_settings;
mod server;
mod storage_backend;
mod tasks;

pub use handlers::admin as admin_handlers;
pub use handlers::auth as auth_handlers;
pub use handlers::images as image_handlers;
pub use handlers::images_cursor;

use bootstrap::{build_app_state, init_logging};
use config::Config;
use router::create_app_with_middleware;
use server::{bind_listener, spawn_cleanup_tasks, start_server};
use tower_http::trace::TraceLayer;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_level = init_logging();
    let config = Config::from_env();

    if let Err(validation_error) = config.validate() {
        error!("Configuration validation failed: {}", validation_error);
        return Err(validation_error.into());
    }

    info!("Configuration loaded (log level: {})", log_level);

    let state = build_app_state(config.clone()).await?;
    spawn_cleanup_tasks(&state);

    let app = create_app_with_middleware(state.clone(), &config, config.server.max_upload_size)
        .layer(TraceLayer::new_for_http());
    let listener = bind_listener(config.addr()).await?;
    start_server(listener, app).await
}
