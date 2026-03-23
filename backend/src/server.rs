//! 服务器启动模块
//! 负责服务器的初始化和启动

use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;

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
