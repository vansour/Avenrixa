//! 服务器启动模块
//! 负责服务器的初始化和启动

use crate::db::AppState;
use crate::domain::auth::state_repository::AuthStateRepository;
use crate::{db::ADMIN_USER_ID, tasks};
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
    let deleted_cleanup_interval = config.cleanup.deleted_cleanup_interval_seconds;
    let expiry_check_interval = config.cleanup.expiry_check_interval_seconds;
    let cleanup_pool = state.postgres_pool_owned().ok();

    // 启动历史已删除图片清理任务（兼容移除回收站前遗留的数据）
    if cleanup_pool.is_none() && state.admin_domain_service.is_none() {
        info!("Skipping background cleanup tasks because no cleanup executor is available",);
        return;
    }
    let cleanup_storage_path = state
        .storage_manager
        .active_settings()
        .local_storage_path
        .clone();
    let expiry_storage_path = cleanup_storage_path.clone();
    let cleanup_retention_days = config.cleanup.deleted_images_retention_days;
    let admin_service_for_cleanup = state.admin_domain_service.clone();
    let installed_state_for_cleanup = state.database.clone();

    let cleanup_pool_for_deleted = cleanup_pool.clone();
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(deleted_cleanup_interval));
        loop {
            interval.tick().await;
            if !crate::db::is_app_installed(&installed_state_for_cleanup)
                .await
                .unwrap_or(false)
            {
                continue;
            }
            info!("Running deleted-image purge task...");
            if let Some(service) = admin_service_for_cleanup.as_ref() {
                if let Err(e) = service.cleanup_deleted_files(ADMIN_USER_ID, "system").await {
                    tracing::error!("Deleted-image purge task failed: {}", e);
                }
            } else if let Some(pool) = cleanup_pool_for_deleted.as_ref() {
                if let Err(e) = tasks::cleanup_expired_images(
                    pool,
                    cleanup_retention_days,
                    &cleanup_storage_path,
                )
                .await
                {
                    tracing::error!("Deleted-image purge task failed: {}", e);
                }
            } else {
                tracing::warn!("Deleted-image purge task skipped: no executor available");
            }
        }
    });

    // 启动过期检查任务（每小时）
    let expiry_pool = cleanup_pool;
    let admin_service_for_expiry = state.admin_domain_service.clone();
    let installed_state_for_expiry = state.database.clone();
    let auth_state_repository = state.auth_state_repository.clone();
    let installed_state_for_auth_cleanup = state.database.clone();

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
            if let Some(service) = admin_service_for_expiry.as_ref() {
                if let Err(e) = service
                    .cleanup_expired_images(ADMIN_USER_ID, "system")
                    .await
                {
                    tracing::error!("Expiry check failed: {}", e);
                }
            } else if let Some(pool) = expiry_pool.as_ref() {
                if let Err(e) = tasks::delete_expired_images(pool, &expiry_storage_path).await {
                    tracing::error!("Expiry check failed: {}", e);
                }
            } else {
                tracing::warn!("Expiry task skipped: no executor available");
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
