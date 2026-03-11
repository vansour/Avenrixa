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
mod sqlite_restore;
mod storage_backend;
mod tasks;

use bootstrap::{BootstrapAppState, BootstrapConfigStore, build_app_state, init_logging};
use config::Config;
use router::{create_app_with_middleware, create_bootstrap_app};
use server::{bind_listener, spawn_cleanup_tasks, start_server};
use sqlite_restore::{
    StartupRestoreOutcome, apply_pending_restore_if_any, finalize_restore_rollback,
    finalize_restore_success, record_startup_restore_failure, rollback_failed_restore,
};
use tower_http::trace::TraceLayer;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_level = init_logging();
    let base_config = Config::from_env();
    let bootstrap_store = BootstrapConfigStore::from_env();
    let bootstrap_file = bootstrap_store.load().await?;
    let runtime_config = bootstrap_store
        .resolve_runtime_database_config(base_config.clone(), bootstrap_file.as_ref());

    info!("Configuration loaded (log level: {})", log_level);

    let listener = bind_listener(base_config.addr()).await?;

    if runtime_config.database.url.trim().is_empty() {
        let app = create_bootstrap_app(
            BootstrapAppState {
                config: base_config.clone(),
                store: std::sync::Arc::new(bootstrap_store),
                runtime_error: None,
                started_at: std::time::Instant::now(),
            },
            &base_config,
            base_config.server.max_upload_size,
        )
        .layer(TraceLayer::new_for_http());
        return start_server(listener, app).await;
    }

    if let Err(validation_error) = runtime_config.validate() {
        error!("Configuration validation failed: {}", validation_error);
        return Err(validation_error.into());
    }

    let startup_restore = apply_pending_restore_if_any(&runtime_config).await?;

    match build_app_state(runtime_config.clone()).await {
        Ok(state) => {
            match &startup_restore {
                StartupRestoreOutcome::None => {}
                StartupRestoreOutcome::StartupFailure(result) => {
                    let _ = record_startup_restore_failure(&state, result).await;
                }
                StartupRestoreOutcome::Applied(context) => {
                    let _ = finalize_restore_success(&state, context).await;
                }
            }
            spawn_cleanup_tasks(&state);
            let app = create_app_with_middleware(
                state.clone(),
                &runtime_config,
                runtime_config.server.max_upload_size,
            )
            .layer(TraceLayer::new_for_http());
            start_server(listener, app).await
        }
        Err(runtime_error) => {
            if let StartupRestoreOutcome::Applied(context) = &startup_restore {
                match rollback_failed_restore(&runtime_config, context, &runtime_error).await {
                    Ok(rollback_result) => match build_app_state(runtime_config.clone()).await {
                        Ok(state) => {
                            let _ = finalize_restore_rollback(&state, &rollback_result).await;
                            spawn_cleanup_tasks(&state);
                            let app = create_app_with_middleware(
                                state.clone(),
                                &runtime_config,
                                runtime_config.server.max_upload_size,
                            )
                            .layer(TraceLayer::new_for_http());
                            return start_server(listener, app).await;
                        }
                        Err(rollback_start_error) => {
                            error!(
                                "Database restore rollback succeeded but application startup still failed: {}",
                                rollback_start_error
                            );
                            let app = create_bootstrap_app(
                                BootstrapAppState {
                                    config: base_config.clone(),
                                    store: std::sync::Arc::new(bootstrap_store),
                                    runtime_error: Some(format!(
                                        "数据库恢复失败后已自动回滚，但应用仍无法启动。原始错误: {}; 回滚后错误: {}",
                                        runtime_error, rollback_start_error
                                    )),
                                    started_at: std::time::Instant::now(),
                                },
                                &base_config,
                                base_config.server.max_upload_size,
                            )
                            .layer(TraceLayer::new_for_http());
                            return start_server(listener, app).await;
                        }
                    },
                    Err(rollback_error) => {
                        error!("Database restore rollback failed: {}", rollback_error);
                        let app = create_bootstrap_app(
                            BootstrapAppState {
                                config: base_config.clone(),
                                store: std::sync::Arc::new(bootstrap_store),
                                runtime_error: Some(format!(
                                    "数据库恢复失败，且自动回滚也失败。原始错误: {}; 回滚错误: {}",
                                    runtime_error, rollback_error
                                )),
                                started_at: std::time::Instant::now(),
                            },
                            &base_config,
                            base_config.server.max_upload_size,
                        )
                        .layer(TraceLayer::new_for_http());
                        return start_server(listener, app).await;
                    }
                }
            }

            error!(
                "Runtime initialization failed, falling back to bootstrap mode: {}",
                runtime_error
            );
            let app = create_bootstrap_app(
                BootstrapAppState {
                    config: base_config.clone(),
                    store: std::sync::Arc::new(bootstrap_store),
                    runtime_error: Some(runtime_error.to_string()),
                    started_at: std::time::Instant::now(),
                },
                &base_config,
                base_config.server.max_upload_size,
            )
            .layer(TraceLayer::new_for_http());
            start_server(listener, app).await
        }
    }
}
