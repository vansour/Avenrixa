use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::database::test_sqlite_connection;
use crate::config::{Config, ConfigError, DatabaseKind};
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
    pub database_max_connections: Option<u32>,
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
        Self {
            path: Arc::new(path),
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
        fallback_max_connections: u32,
    ) -> anyhow::Result<UpdateBootstrapDatabaseConfigResponse> {
        let database_kind = DatabaseKind::parse(&req.database_kind)
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        let database_url = normalize_database_target(database_kind, &req.database_url)
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;

        let max_connections = req
            .database_max_connections
            .unwrap_or(fallback_max_connections)
            .max(1);

        match database_kind {
            DatabaseKind::Postgres => {
                test_postgres_connection(&database_url, max_connections).await?
            }
            DatabaseKind::Sqlite => test_sqlite_connection(&database_url).await?,
        }

        let payload = BootstrapConfigFile {
            database_kind,
            database_url,
            database_max_connections: Some(max_connections),
        };

        if let Some(parent) = self.path().parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let serialized = serde_json::to_string_pretty(&payload)?;
        tokio::fs::write(self.path(), serialized).await?;

        Ok(UpdateBootstrapDatabaseConfigResponse {
            database_kind: payload.database_kind.as_str().to_string(),
            database_configured: true,
            database_url_masked: mask_database_url(payload.database_kind, &payload.database_url),
            database_max_connections: max_connections,
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
            if let Some(max_connections) = file.database_max_connections {
                config.database.max_connections = max_connections.max(1);
            }
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
                .map(|file| file.database_kind.as_str().to_string())
                .unwrap_or_else(|| config.database.kind.as_str().to_string()),
            database_configured: file.is_some(),
            database_url_masked: file
                .map(|file| mask_database_url(file.database_kind, &file.database_url)),
            database_max_connections: file
                .and_then(|file| file.database_max_connections)
                .or(Some(config.database.max_connections)),
            restart_required: file.is_some(),
            runtime_error,
        }
    }

    pub fn runtime_status(&self, config: &Config) -> BootstrapStatusResponse {
        BootstrapStatusResponse {
            mode: "runtime".to_string(),
            database_kind: config.database.kind.as_str().to_string(),
            database_configured: !config.database.url.trim().is_empty(),
            database_url_masked: (!config.database.url.trim().is_empty())
                .then(|| mask_database_url(config.database.kind, &config.database.url)),
            database_max_connections: Some(config.database.max_connections),
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
        DatabaseKind::Postgres => match reqwest::Url::parse(database_url) {
            Ok(mut url) => {
                let username = url.username().to_string();
                if !username.is_empty() {
                    let _ = url.set_username(&username);
                }
                if url.password().is_some() {
                    let _ = url.set_password(Some("******"));
                }
                url.to_string()
            }
            Err(_) => "******".to_string(),
        },
        DatabaseKind::Sqlite => database_url.to_string(),
    }
}
