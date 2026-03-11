use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use crate::config::{Config, DatabaseKind, normalize_mysql_compatible_url};
use crate::db;
use crate::db::DatabasePool;
use sqlx::mysql::{MySqlPoolOptions, MySqlSslMode};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use tracing::info;

pub async fn initialize_database(config: &Config) -> anyhow::Result<DatabasePool> {
    match config.database.kind {
        DatabaseKind::Postgres => {
            info!("Connecting to PostgreSQL database...");
            let pool = PgPoolOptions::new()
                .max_connections(config.database.max_connections)
                .connect(&config.database.url)
                .await?;

            info!("Running PostgreSQL database migrations...");
            let database = DatabasePool::Postgres(pool);
            db::run_migrations(&database).await?;

            Ok(database)
        }
        DatabaseKind::MySql => {
            info!("Connecting to MySQL database...");
            let database_url = normalize_mysql_compatible_url(&config.database.url);
            let pool = mysql_pool_options(config.database.max_connections)
                .connect(&database_url)
                .await?;

            info!("Running MySQL database migrations...");
            let database = DatabasePool::MySql(pool);
            db::run_migrations(&database).await?;

            Ok(database)
        }
        DatabaseKind::Sqlite => {
            info!("Connecting to SQLite database...");
            let pool = SqlitePoolOptions::new()
                .max_connections(config.database.max_connections)
                .connect_with(sqlite_connect_options(&config.database.url).await?)
                .await?;

            info!("Running SQLite database migrations...");
            let database = DatabasePool::Sqlite(pool);
            db::run_migrations(&database).await?;

            Ok(database)
        }
    }
}

pub fn mysql_pool_options(max_connections: u32) -> MySqlPoolOptions {
    MySqlPoolOptions::new()
        .max_connections(max_connections.max(1))
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("SET time_zone = '+00:00'")
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
}

pub async fn test_mysql_connection(database_url: &str, max_connections: u32) -> anyhow::Result<()> {
    let options =
        sqlx::mysql::MySqlConnectOptions::from_str(&normalize_mysql_compatible_url(database_url))?
            .ssl_mode(MySqlSslMode::Preferred);
    let pool = mysql_pool_options(max_connections)
        .connect_with(options)
        .await?;
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await?;
    pool.close().await;
    Ok(())
}

pub async fn test_sqlite_connection(database_url: &str) -> anyhow::Result<()> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlite_connect_options(database_url).await?)
        .await?;
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await?;
    pool.close().await;
    Ok(())
}

pub async fn sqlite_connect_options(database_url: &str) -> anyhow::Result<SqliteConnectOptions> {
    let trimmed = database_url.trim();
    if trimmed.is_empty() {
        anyhow::bail!("SQLite 数据库地址不能为空");
    }

    if let Some(parent) = sqlite_database_parent(trimmed) {
        tokio::fs::create_dir_all(parent).await?;
    }

    let options = if trimmed.starts_with("sqlite:") {
        SqliteConnectOptions::from_str(trimmed)?
    } else {
        SqliteConnectOptions::new().filename(trimmed)
    };

    Ok(options
        .create_if_missing(true)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5))
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal))
}

fn sqlite_database_parent(database_url: &str) -> Option<PathBuf> {
    resolve_sqlite_database_path(database_url)
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .and_then(|parent| {
            if parent.as_os_str().is_empty() {
                None
            } else {
                Some(parent)
            }
        })
}

pub fn resolve_sqlite_database_path(database_url: &str) -> anyhow::Result<PathBuf> {
    let trimmed = database_url.trim();
    if trimmed.is_empty() {
        anyhow::bail!("SQLite 数据库地址不能为空");
    }

    if let Some(path) = trimmed.strip_prefix("sqlite://") {
        if path.is_empty() || path == ":memory:" || path.starts_with('?') || path.contains('?') {
            anyhow::bail!("当前 SQLite 连接不是受支持的文件型数据库地址");
        }
        Ok(Path::new(path).to_path_buf())
    } else if let Some(path) = trimmed.strip_prefix("sqlite:") {
        if path.is_empty() || path == ":memory:" || path.starts_with('?') || path.contains('?') {
            anyhow::bail!("当前 SQLite 连接不是受支持的文件型数据库地址");
        }
        Ok(Path::new(path).to_path_buf())
    } else if !trimmed.contains("://") {
        if trimmed == ":memory:" {
            anyhow::bail!("当前 SQLite 连接不是受支持的文件型数据库地址");
        }
        Ok(Path::new(trimmed).to_path_buf())
    } else {
        anyhow::bail!("当前 SQLite 连接不是受支持的文件型数据库地址")
    }
}
