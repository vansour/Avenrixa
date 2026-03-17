//! 服务器启动模块
//! 负责服务器的初始化和启动

use crate::db::ADMIN_USER_ID;
use crate::db::AppState;
use crate::domain::auth::state_repository::AuthStateRepository;
use crate::storage_backend::process_pending_storage_cleanup_jobs;
use chrono::Utc;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;

/// 启动清理任务
pub fn spawn_cleanup_tasks(state: &AppState) {
    let config = &state.config;
    if !config.cleanup.enabled {
        info!("Background cleanup tasks are disabled by configuration");
        return;
    }
    let expiry_check_interval = config.cleanup.expiry_check_interval_seconds;
    let admin_service_for_expiry = state.admin_domain_service.clone();
    let installed_state_for_expiry = state.database.clone();
    let auth_state_repository = state.auth_state_repository.clone();
    let installed_state_for_auth_cleanup = state.database.clone();
    let storage_cleanup_database = state.database.clone();
    let installed_state_for_storage_cleanup = state.database.clone();
    let storage_cleanup_interval = expiry_check_interval.clamp(5, 30);

    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(expiry_check_interval));
        loop {
            interval.tick().await;
            if !crate::db::is_app_installed(&installed_state_for_expiry)
                .await
                .unwrap_or(false)
            {
                continue;
            }
            if let Err(error) = admin_service_for_expiry
                .cleanup_expired_images(ADMIN_USER_ID, "system")
                .await
            {
                tracing::error!("Expiry check failed: {}", error);
            }
        }
    });

    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(expiry_check_interval));
        loop {
            interval.tick().await;
            if !crate::db::is_app_installed(&installed_state_for_auth_cleanup)
                .await
                .unwrap_or(false)
            {
                continue;
            }
            if let Err(error) = auth_state_repository
                .purge_expired_revoked_tokens(Utc::now())
                .await
            {
                tracing::error!("Revoked token cleanup failed: {}", error);
            }
        }
    });

    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(storage_cleanup_interval));
        loop {
            interval.tick().await;
            if !crate::db::is_app_installed(&installed_state_for_storage_cleanup)
                .await
                .unwrap_or(false)
            {
                continue;
            }
            if let Err(error) =
                process_pending_storage_cleanup_jobs(&storage_cleanup_database, 64).await
            {
                tracing::error!("Storage cleanup retry failed: {}", error);
            }
        }
    });
}

/// 启动服务器
pub async fn start_server(listener: TcpListener, app: axum::Router) -> anyhow::Result<()> {
    info!("Server listening on {}", listener.local_addr()?);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

/// 绑定监听地址
pub async fn bind_listener(addr: SocketAddr) -> anyhow::Result<TcpListener> {
    let listener = TcpListener::bind(addr).await?;
    Ok(listener)
}
