use fred::prelude::{
    Builder, ClientLike, Error as FredError, Expiration, KeysInterface, Pool, ReconnectPolicy,
    Server, ServerConfig, TlsConnector,
};
use reqwest::Url;

pub type CacheBackendError = FredError;

#[derive(Clone)]
pub struct CacheConnection {
    client: Pool,
}

impl CacheConnection {
    pub async fn connect(cache_url: &str) -> anyhow::Result<Self> {
        let config = parse_cache_config(cache_url)?;
        let client = Builder::from_config(config)
            .set_policy(ReconnectPolicy::new_exponential(0, 100, 30_000, 2))
            .build_pool(2)
            .map_err(|error| anyhow::anyhow!("Failed to build Dragonfly client pool: {}", error))?;

        let _connection_task = client
            .init()
            .await
            .map_err(|error| anyhow::anyhow!("Failed to connect to Dragonfly: {}", error))?;

        Ok(Self { client })
    }

    pub async fn ping(&self) -> Result<(), CacheBackendError> {
        self.client.ping::<String>(None).await.map(|_| ())
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>, CacheBackendError> {
        self.client.get(key).await
    }

    pub async fn set_string(
        &self,
        key: &str,
        value: impl Into<String>,
        ttl_seconds: u64,
    ) -> Result<(), CacheBackendError> {
        let ttl_seconds = i64::try_from(ttl_seconds).map_err(|_| {
            FredError::new(
                fred::error::ErrorKind::InvalidArgument,
                "Cache TTL exceeds the supported range",
            )
        })?;

        self.client
            .set::<(), _, _>(
                key,
                value.into(),
                Some(Expiration::EX(ttl_seconds)),
                None,
                false,
            )
            .await
    }

    pub async fn del(&self, key: &str) -> Result<(), CacheBackendError> {
        self.client.del::<i64, _>(key).await.map(|_| ())
    }

    pub async fn del_many(&self, keys: Vec<String>) -> Result<(), CacheBackendError> {
        self.client.del::<i64, _>(keys).await.map(|_| ())
    }

    pub async fn scan_page(
        &self,
        cursor: &str,
        pattern: &str,
        count: u32,
    ) -> Result<(String, Vec<String>), CacheBackendError> {
        self.client
            .scan_page(cursor.to_string(), pattern.to_string(), Some(count), None)
            .await
    }
}

fn parse_cache_config(cache_url: &str) -> anyhow::Result<fred::prelude::Config> {
    let cache_url = cache_url.trim();
    let parsed = Url::parse(cache_url)
        .map_err(|error| anyhow::anyhow!("Invalid Dragonfly URL '{}': {}", cache_url, error))?;
    let scheme = parsed.scheme();
    let tls_enabled = match scheme {
        "dragonfly" => false,
        "dragonflys" => true,
        other => {
            return Err(anyhow::anyhow!(
                "Unsupported CACHE_URL scheme '{}', expected dragonfly:// or dragonflys://",
                other
            ));
        }
    };

    let host = parsed
        .host_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("CACHE_URL is missing a Dragonfly host"))?
        .to_string();
    let port = parsed.port().unwrap_or(6379);
    let database = parse_database(&parsed)?;

    let server = if tls_enabled {
        Server::new_with_tls(host.clone(), port, Some(host.clone()))
    } else {
        Server::new(host.clone(), port)
    };

    Ok(fred::prelude::Config {
        server: ServerConfig::Centralized { server },
        username: optional_string(parsed.username()),
        password: parsed.password().map(ToOwned::to_owned),
        database,
        tls: if tls_enabled {
            Some(
                build_tls_connector()
                    .map_err(|error| {
                        anyhow::anyhow!("Failed to initialize Dragonfly TLS: {}", error)
                    })?
                    .into(),
            )
        } else {
            None
        },
        ..Default::default()
    })
}

fn optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_database(url: &Url) -> anyhow::Result<Option<u8>> {
    let path = url.path().trim();
    if path.is_empty() || path == "/" {
        return Ok(None);
    }

    let database = path.trim_start_matches('/');
    if database.is_empty() || database.contains('/') {
        return Err(anyhow::anyhow!(
            "CACHE_URL database path must be a single integer segment"
        ));
    }

    database
        .parse::<u8>()
        .map(Some)
        .map_err(|error| anyhow::anyhow!("Invalid Dragonfly database '{}': {}", database, error))
}

fn build_tls_connector() -> Result<TlsConnector, CacheBackendError> {
    let _ = fred::rustls::crypto::ring::default_provider().install_default();
    TlsConnector::default_rustls()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cache_config_supports_plain_dragonfly_urls() {
        let config = parse_cache_config("dragonfly://cache.internal:6380/4").unwrap();

        assert_eq!(
            config,
            fred::prelude::Config {
                server: ServerConfig::Centralized {
                    server: Server::new("cache.internal", 6380),
                },
                database: Some(4),
                ..Default::default()
            }
        );
    }

    #[test]
    fn parse_cache_config_supports_tls_and_credentials() {
        let config = parse_cache_config("dragonflys://user:secret@cache.internal/2").unwrap();

        assert_eq!(config.username.as_deref(), Some("user"));
        assert_eq!(config.password.as_deref(), Some("secret"));
        assert_eq!(config.database, Some(2));
        assert!(config.uses_tls());
    }

    #[test]
    fn parse_cache_config_rejects_non_dragonfly_schemes() {
        let error = parse_cache_config("https://cache.internal").unwrap_err();

        assert!(
            error
                .to_string()
                .contains("expected dragonfly:// or dragonflys://")
        );
    }
}
