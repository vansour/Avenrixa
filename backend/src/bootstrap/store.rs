use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::database::{test_mysql_connection, test_sqlite_connection};
use crate::config::{
    Config, ConfigError, DatabaseKind, is_mysql_compatible_scheme, normalize_mysql_compatible_url,
};
use crate::models::{
    BootstrapStatusResponse, UpdateBootstrapDatabaseConfigRequest,
    UpdateBootstrapDatabaseConfigResponse,
};

const DEFAULT_BOOTSTRAP_CONFIG_PATH: &str = "/data/bootstrap/config.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BootstrapConfigFile {
    #[serde(default)]
    pub database_kind: DatabaseKind,
    pub database_url: String,
}

#[derive(Clone)]
pub struct BootstrapConfigStore {
    path: Arc<PathBuf>,
}

impl BootstrapConfigStore {
    pub fn from_env() -> Self {
        let path = std::env::var("BOOTSTRAP_CONFIG_PATH")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_BOOTSTRAP_CONFIG_PATH));
        Self::from_path(path)
    }

    pub(crate) fn from_path(path: impl Into<PathBuf>) -> Self {
        Self {
            path: Arc::new(path.into()),
        }
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub async fn load(&self) -> anyhow::Result<Option<BootstrapConfigFile>> {
        let path = self.path();
        if !tokio::fs::try_exists(path).await? {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(path).await?;
        let parsed: BootstrapConfigFile = serde_json::from_str(&content)?;
        if parsed.database_url.trim().is_empty() {
            return Ok(None);
        }

        Ok(Some(parsed))
    }

    pub async fn save_database_config(
        &self,
        req: &UpdateBootstrapDatabaseConfigRequest,
        connection_test_max_connections: u32,
    ) -> anyhow::Result<UpdateBootstrapDatabaseConfigResponse> {
        let database_url = normalize_database_target(req.database_kind, &req.database_url)
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;

        let max_connections = connection_test_max_connections.max(1);

        match req.database_kind {
            DatabaseKind::Postgres => {
                test_postgres_connection(&database_url, max_connections).await?
            }
            DatabaseKind::MySql => test_mysql_connection(&database_url, max_connections).await?,
            DatabaseKind::Sqlite => test_sqlite_connection(&database_url).await?,
        }

        let payload = BootstrapConfigFile {
            database_kind: req.database_kind,
            database_url,
        };

        if let Some(parent) = self.path().parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let serialized = serde_json::to_string_pretty(&payload)?;
        tokio::fs::write(self.path(), serialized).await?;

        Ok(UpdateBootstrapDatabaseConfigResponse {
            database_kind: payload.database_kind,
            database_configured: true,
            database_url_masked: mask_database_url(payload.database_kind, &payload.database_url),
            restart_required: true,
        })
    }

    pub fn resolve_runtime_database_config(
        &self,
        mut config: Config,
        file: Option<&BootstrapConfigFile>,
    ) -> Config {
        if config.database.url.trim().is_empty()
            && let Some(file) = file
            && !file.database_url.trim().is_empty()
        {
            config.database.kind = file.database_kind;
            config.database.url = file.database_url.trim().to_string();
        }
        config
    }

    pub fn bootstrap_status(
        &self,
        config: &Config,
        file: Option<&BootstrapConfigFile>,
        runtime_error: Option<String>,
    ) -> BootstrapStatusResponse {
        BootstrapStatusResponse {
            mode: "bootstrap".to_string(),
            database_kind: file
                .map(|file| file.database_kind)
                .unwrap_or(config.database.kind),
            database_configured: file.is_some(),
            database_url_masked: file
                .map(|file| mask_database_url(file.database_kind, &file.database_url)),
            cache_configured: config.cache_backend.url.is_some(),
            cache_url_masked: config.cache_backend.url.as_deref().map(mask_cache_url),
            restart_required: file.is_some(),
            runtime_error,
        }
    }

    pub fn runtime_status(&self, config: &Config) -> BootstrapStatusResponse {
        BootstrapStatusResponse {
            mode: "runtime".to_string(),
            database_kind: config.database.kind,
            database_configured: !config.database.url.trim().is_empty(),
            database_url_masked: (!config.database.url.trim().is_empty())
                .then(|| mask_database_url(config.database.kind, &config.database.url)),
            cache_configured: config
                .cache_backend
                .url
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty()),
            cache_url_masked: config
                .cache_backend
                .url
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(mask_cache_url),
            restart_required: false,
            runtime_error: None,
        }
    }
}

async fn test_postgres_connection(database_url: &str, max_connections: u32) -> anyhow::Result<()> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(max_connections.max(1))
        .connect(database_url)
        .await?;
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await?;
    pool.close().await;
    Ok(())
}

fn normalize_database_target(
    database_kind: DatabaseKind,
    value: &str,
) -> Result<String, ConfigError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ConfigError::DatabaseUrlEmpty);
    }

    match database_kind {
        DatabaseKind::Postgres => {
            if !matches!(
                trimmed.split(':').next(),
                Some("postgresql") | Some("postgres")
            ) {
                return Err(ConfigError::InvalidPostgresDatabaseUrl);
            }
            Ok(trimmed.to_string())
        }
        DatabaseKind::MySql => {
            if !is_mysql_compatible_scheme(trimmed) {
                return Err(ConfigError::InvalidMySqlDatabaseUrl);
            }
            Ok(normalize_mysql_compatible_url(trimmed))
        }
        DatabaseKind::Sqlite => {
            if trimmed.starts_with("sqlite:") || !trimmed.contains("://") {
                Ok(trimmed.to_string())
            } else {
                Err(ConfigError::InvalidSqliteDatabaseUrl)
            }
        }
    }
}

pub fn mask_database_url(database_kind: DatabaseKind, database_url: &str) -> String {
    match database_kind {
        DatabaseKind::Postgres => "postgresql://******".to_string(),
        DatabaseKind::MySql => {
            if database_url
                .trim()
                .to_ascii_lowercase()
                .starts_with("mariadb://")
            {
                "mariadb://******".to_string()
            } else {
                "mysql://******".to_string()
            }
        }
        DatabaseKind::Sqlite => {
            if database_url
                .trim()
                .to_ascii_lowercase()
                .starts_with("sqlite://")
            {
                "sqlite://******".to_string()
            } else {
                "******".to_string()
            }
        }
    }
}

pub fn mask_cache_url(cache_url: &str) -> String {
    let trimmed = cache_url.trim().to_ascii_lowercase();
    if trimmed.starts_with("rediss://") {
        "rediss://******".to_string()
    } else if trimmed.starts_with("redis://") {
        "redis://******".to_string()
    } else {
        "******".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_database(kind: DatabaseKind, url: &str) -> Config {
        let mut config = Config::default();
        config.database.kind = kind;
        config.database.url = url.to_string();
        config
    }

    #[test]
    fn resolve_runtime_database_config_uses_bootstrap_file_when_env_is_missing() {
        let store = BootstrapConfigStore::from_env();
        let config = Config::default();
        let file = BootstrapConfigFile {
            database_kind: DatabaseKind::Sqlite,
            database_url: "sqlite:///data/sqlite/app.db".to_string(),
        };

        let resolved = store.resolve_runtime_database_config(config, Some(&file));

        assert_eq!(resolved.database.kind, DatabaseKind::Sqlite);
        assert_eq!(resolved.database.url, "sqlite:///data/sqlite/app.db");
    }

    #[test]
    fn resolve_runtime_database_config_keeps_env_database_when_present() {
        let store = BootstrapConfigStore::from_env();
        let config = config_with_database(
            DatabaseKind::Postgres,
            "postgresql://user:pass@postgres:5432/image",
        );
        let file = BootstrapConfigFile {
            database_kind: DatabaseKind::Sqlite,
            database_url: "sqlite:///data/sqlite/app.db".to_string(),
        };

        let resolved = store.resolve_runtime_database_config(config.clone(), Some(&file));

        assert_eq!(resolved.database.kind, config.database.kind);
        assert_eq!(resolved.database.url, config.database.url);
    }

    #[test]
    fn bootstrap_status_masks_saved_database_target_without_leaking_value() {
        let store = BootstrapConfigStore::from_env();
        let config = Config::default();
        let file = BootstrapConfigFile {
            database_kind: DatabaseKind::Sqlite,
            database_url: "/srv/app/private.sqlite3".to_string(),
        };

        let status = store.bootstrap_status(&config, Some(&file), None);

        assert_eq!(status.mode, "bootstrap");
        assert_eq!(status.database_kind, DatabaseKind::Sqlite);
        assert_eq!(status.database_url_masked.as_deref(), Some("******"));
        assert!(!status.cache_configured);
        assert_eq!(status.cache_url_masked, None);
        assert!(status.database_configured);
        assert!(status.restart_required);
    }

    #[test]
    fn runtime_status_masks_database_target_without_leaking_value() {
        let store = BootstrapConfigStore::from_env();
        let config = config_with_database(
            DatabaseKind::MySql,
            "mysql://user:pass@mysql.internal:3306/image",
        );

        let status = store.runtime_status(&config);

        assert_eq!(status.mode, "runtime");
        assert_eq!(status.database_kind, DatabaseKind::MySql);
        assert_eq!(
            status.database_url_masked.as_deref(),
            Some("mysql://******")
        );
        assert!(!status.cache_configured);
        assert_eq!(status.cache_url_masked, None);
        assert!(status.database_configured);
        assert!(!status.restart_required);
    }

    #[test]
    fn mask_database_url_preserves_only_scheme_family() {
        assert_eq!(
            mask_database_url(
                DatabaseKind::Postgres,
                "postgresql://user:pass@postgres:5432/image"
            ),
            "postgresql://******"
        );
        assert_eq!(
            mask_database_url(DatabaseKind::MySql, "mariadb://user:pass@mysql:3306/image"),
            "mariadb://******"
        );
        assert_eq!(
            mask_database_url(DatabaseKind::Sqlite, "sqlite:///data/sqlite/app.db"),
            "sqlite://******"
        );
    }

    #[test]
    fn mask_cache_url_preserves_only_scheme_family() {
        assert_eq!(mask_cache_url("redis://cache:6379"), "redis://******");
        assert_eq!(
            mask_cache_url("rediss://user:pass@cache.example.com:6380/0"),
            "rediss://******"
        );
        assert_eq!(mask_cache_url("cache.internal"), "******");
    }
}
