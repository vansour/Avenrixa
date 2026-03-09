//! 服务器启动模块
//! 负责服务器的初始化和启动

use crate::db::AppState;
use crate::{db::ADMIN_USER_ID, tasks};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;

/// 启动清理任务
#[allow(dead_code)]
pub fn spawn_cleanup_tasks(state: &AppState) {
    let config = &state.config;
    if !config.cleanup.enabled {
        info!("Background cleanup tasks are disabled by configuration");
        return;
    }
    let deleted_cleanup_interval = config.cleanup.deleted_cleanup_interval_seconds;
    let expiry_check_interval = config.cleanup.expiry_check_interval_seconds;

    // 启动清理已删除文件任务（每天）
    let cleanup_pool = state.pool.clone();
    let cleanup_storage_path = config.storage.path.clone();
    let cleanup_thumbnail_path = config.storage.thumbnail_path.clone();
    let cleanup_retention_days = config.cleanup.deleted_images_retention_days;
    let admin_service_for_cleanup = state.admin_domain_service.clone();

    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(deleted_cleanup_interval));
        loop {
            interval.tick().await;
            info!("Running cleanup task...");
            if let Some(service) = admin_service_for_cleanup.as_ref() {
                if let Err(e) = service.cleanup_deleted_files(ADMIN_USER_ID, "system").await {
                    tracing::error!("Cleanup task failed: {}", e);
                }
            } else if let Err(e) = tasks::cleanup_expired_images(
                &cleanup_pool,
                cleanup_retention_days,
                &cleanup_storage_path,
                &cleanup_thumbnail_path,
            )
            .await
            {
                tracing::error!("Cleanup task failed: {}", e);
            }
        }
    });

    // 启动过期检查任务（每小时）
    let expiry_pool = state.pool.clone();
    let admin_service_for_expiry = state.admin_domain_service.clone();

    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(expiry_check_interval));
        loop {
            interval.tick().await;
            if let Some(service) = admin_service_for_expiry.as_ref() {
                if let Err(e) = service.cleanup_expired_images(ADMIN_USER_ID).await {
                    tracing::error!("Expiry check failed: {}", e);
                }
            } else if let Err(e) = tasks::move_expired_to_trash(&expiry_pool).await {
                tracing::error!("Expiry check failed: {}", e);
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
