use crate::cache::CacheConnection;
use crate::config::Config;
use redis::Client;
use tracing::{info, warn};

pub struct CacheConnections {
    pub app: Option<CacheConnection>,
}

pub async fn connect_cache(config: &Config) -> CacheConnections {
    let Some(configured_cache_url) = config
        .cache_backend
        .url
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    else {
        info!("REDIS_URL not configured, starting without external cache");
        return CacheConnections { app: None };
    };

    info!("Connecting to external cache...");
    match Client::open(configured_cache_url.to_string()) {
        Ok(cache_client) => match cache_client.get_connection_manager().await {
            Ok(cache_connection) => CacheConnections {
                app: Some(cache_connection),
            },
            Err(error) => {
                warn!(
                    "External cache connection failed, falling back to cache-disabled mode: {}",
                    error
                );
                CacheConnections { app: None }
            }
        },
        Err(error) => {
            warn!(
                "Invalid REDIS_URL for external cache, falling back to cache-disabled mode: {}",
                error
            );
            CacheConnections { app: None }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_cache_without_url_disables_cache() {
        let config = Config::default();

        let connections = connect_cache(&config).await;

        assert!(connections.app.is_none());
    }

    #[tokio::test]
    async fn connect_cache_with_invalid_url_degrades_cache() {
        let mut config = Config::default();
        config.cache_backend.url = Some("not a redis url".to_string());

        let connections = connect_cache(&config).await;

        assert!(connections.app.is_none());
    }
}
