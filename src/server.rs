//! 服务器启动模块
//! 负责服务器的初始化和启动

use crate::db::AppState;
use crate::tasks;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;

/// 启动清理任务
pub fn spawn_cleanup_tasks(state: &AppState) {
    let config = &state.config;

    // 启动清理过期图片任务（每天）
    let cleanup_pool = state.pool.clone();
    let cleanup_storage_path = config.storage.path.clone();
    let cleanup_retention_days = config.cleanup.deleted_images_retention_days;

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(86400));
        loop {
            interval.tick().await;
            info!("Running cleanup task...");
            if let Err(e) = tasks::cleanup_expired_images(
                &cleanup_pool,
                cleanup_retention_days,
                &cleanup_storage_path,
            )
            .await
            {
                tracing::error!("Cleanup task failed: {}", e);
            }
        }
    });

    // 启动过期检查任务（每小时）
    let expiry_pool = state.pool.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            if let Err(e) = tasks::move_expired_to_trash(&expiry_pool).await {
                tracing::error!("Expiry check failed: {}", e);
            }
        }
    });
}

/// 启动服务器
pub async fn start_server(listener: TcpListener, app: axum::Router) -> anyhow::Result<()> {
    info!("Server listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}

/// 绑定监听地址
pub async fn bind_listener(addr: SocketAddr) -> anyhow::Result<TcpListener> {
    let listener = TcpListener::bind(addr).await?;
    Ok(listener)
}
