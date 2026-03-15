use crate::cache::CacheConnection;
use crate::config::Config;
use crate::models::ComponentStatus;
use redis::Client;
use tracing::{info, warn};

pub struct CacheConnections {
    pub app: Option<CacheConnection>,
    pub status: ComponentStatus,
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
        return CacheConnections {
            app: None,
            status: ComponentStatus::disabled("未配置外部缓存，运行在无缓存模式"),
        };
    };

    info!("Connecting to external cache...");
    match Client::open(configured_cache_url.to_string()) {
        Ok(cache_client) => match cache_client.get_connection_manager().await {
            Ok(cache_connection) => CacheConnections {
                app: Some(cache_connection),
                status: ComponentStatus::healthy(),
            },
            Err(error) => {
                warn!(
                    "External cache connection failed, falling back to cache-disabled mode: {}",
                    error
                );
                CacheConnections {
                    app: None,
                    status: ComponentStatus::degraded(format!(
                        "外部缓存连接失败，已降级为无缓存模式: {}",
                        error
                    )),
                }
            }
        },
        Err(error) => {
            warn!(
                "Invalid REDIS_URL for external cache, falling back to cache-disabled mode: {}",
                error
            );
            CacheConnections {
                app: None,
                status: ComponentStatus::degraded(format!(
                    "外部缓存地址无效，已降级为无缓存模式: {}",
                    error
                )),
            }
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
        assert_eq!(
            connections.status.status,
            crate::models::HealthState::Disabled
        );
        assert!(
            connections
                .status
                .message
                .as_deref()
                .is_some_and(|message| message.contains("无缓存模式"))
        );
    }

    #[tokio::test]
    async fn connect_cache_with_invalid_url_degrades_cache() {
        let mut config = Config::default();
        config.cache_backend.url = Some("not a redis url".to_string());

        let connections = connect_cache(&config).await;

        assert!(connections.app.is_none());
        assert_eq!(
            connections.status.status,
            crate::models::HealthState::Degraded
        );
        assert!(
            connections
                .status
                .message
                .as_deref()
                .is_some_and(|message| message.contains("地址无效"))
        );
    }
}
