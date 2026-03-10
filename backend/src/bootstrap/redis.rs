use crate::config::Config;
use redis::{Client, aio::ConnectionManager};
use tracing::info;

pub struct RedisConnections {
    pub app: ConnectionManager,
}

pub async fn connect_redis(config: &Config) -> anyhow::Result<RedisConnections> {
    info!("Connecting to Redis...");
    let redis_client = Client::open(config.redis.url.clone())?;

    Ok(RedisConnections {
        app: redis_client.get_connection_manager().await?,
    })
}
