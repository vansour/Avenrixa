mod audit;
mod auth;
mod backup_manifest;
mod bootstrap;
mod cache;
mod config;
mod db;
mod domain;
pub mod error;
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

use bootstrap::{BootstrapAppState, BootstrapConfigStore, build_app_state, init_logging};
use config::Config;
use router::{create_app_with_middleware, create_bootstrap_app};
use server::{bind_listener, spawn_cleanup_tasks, start_server};
use tower_http::trace::TraceLayer;
use tracing::{error, info};

async fn build_application(
    base_config: Config,
    bootstrap_store: BootstrapConfigStore,
) -> anyhow::Result<axum::Router> {
    let bootstrap_file = bootstrap_store.load().await?;
    let runtime_config = bootstrap_store
        .resolve_runtime_database_config(base_config.clone(), bootstrap_file.as_ref());

    if runtime_config.database.url.trim().is_empty() {
        return Ok(create_bootstrap_app(
            BootstrapAppState {
                config: base_config.clone(),
                store: std::sync::Arc::new(bootstrap_store),
                runtime_error: None,
                started_at: std::time::Instant::now(),
            },
            &base_config,
            base_config.server.max_upload_size,
        )
        .layer(TraceLayer::new_for_http()));
    }

    if let Err(validation_error) = runtime_config.validate() {
        error!("Configuration validation failed: {}", validation_error);
        return Err(validation_error.into());
    }
    match build_app_state(runtime_config.clone()).await {
        Ok(state) => {
            spawn_cleanup_tasks(&state);
            Ok(create_app_with_middleware(
                state.clone(),
                &runtime_config,
                runtime_config.server.max_upload_size,
            )
            .layer(TraceLayer::new_for_http()))
        }
        Err(runtime_error) => {
            error!(
                "Runtime initialization failed; refusing to expose bootstrap mode because runtime database is already configured: {}",
                runtime_error
            );
            Err(runtime_error)
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_level = init_logging();
    let base_config = Config::from_env();
    let bootstrap_store = BootstrapConfigStore::from_env();
    let server_addr = base_config.addr();

    info!("Configuration loaded (log level: {})", log_level);

    let app = build_application(base_config, bootstrap_store).await?;
    let listener = bind_listener(server_addr).await?;
    start_server(listener, app).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::BootstrapConfigFile;
    use crate::config::DatabaseKind;
    use std::sync::OnceLock;

    fn ensure_test_env() {
        static INIT: OnceLock<()> = OnceLock::new();
        INIT.get_or_init(|| {
            #[allow(unused_unsafe)]
            unsafe {
                std::env::set_var(
                    "JWT_SECRET",
                    "test-jwt-secret-with-sufficient-length-1234567890",
                );
            }
        });
    }

    #[tokio::test]
    async fn configured_runtime_failure_fails_closed_instead_of_falling_back_to_bootstrap() {
        ensure_test_env();

        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let bootstrap_path = temp_dir.path().join("bootstrap.json");
        let fallback = BootstrapConfigFile {
            database_kind: DatabaseKind::Postgres,
            database_url: "postgresql://user:pass@127.0.0.1:5432/bootstrap_fallback".to_string(),
        };
        std::fs::write(
            &bootstrap_path,
            serde_json::to_vec(&fallback).expect("fallback config should serialize"),
        )
        .expect("fallback config should be written");

        let frontend_dir = temp_dir.path().join("frontend");
        std::fs::create_dir_all(&frontend_dir).expect("frontend dir should be created");
        std::fs::write(frontend_dir.join("index.html"), "<html>test</html>")
            .expect("index should be written");

        let mut config = Config::default();
        config.server.frontend_dir = frontend_dir.to_string_lossy().into_owned();
        config.database.kind = DatabaseKind::Postgres;
        config.database.url = "postgresql://127.0.0.1:9/image".to_string();
        config.storage.path = temp_dir
            .path()
            .join("storage")
            .to_string_lossy()
            .into_owned();

        let error = build_application(config, BootstrapConfigStore::from_path(&bootstrap_path))
            .await
            .expect_err("configured runtime failure should not expose bootstrap router");

        let message = error.to_string();
        assert!(
            !message.contains("bootstrap"),
            "startup should fail closed instead of returning bootstrap router: {message}"
        );
    }
}
