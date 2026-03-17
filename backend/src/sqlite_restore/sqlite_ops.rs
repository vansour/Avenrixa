use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use crate::bootstrap::sqlite_connect_options;
use crate::config::Config;
use crate::db::{ADMIN_USER_ID, DatabasePool, INSTALL_STATE_SETTING_KEY};
use crate::runtime_settings::{RuntimeSettings, load_from_db};

const REQUIRED_BACKUP_TABLES: [&str; 3] = ["users", "settings", "images"];

#[derive(Debug)]
pub(super) struct SqliteBackupInspection {
    pub(super) integrity_check_passed: bool,
    pub(super) app_installed: bool,
    pub(super) has_admin: bool,
    pub(super) missing_tables: Vec<String>,
    pub(super) runtime_settings: RuntimeSettings,
}

pub(super) async fn create_sqlite_snapshot(
    config: &Config,
    target_path: &Path,
) -> anyhow::Result<()> {
    let mut conn = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlite_connect_options(&config.database.url).await?)
        .await?
        .acquire()
        .await?;

    if tokio::fs::try_exists(target_path).await.unwrap_or(false) {
        let _ = tokio::fs::remove_file(target_path).await;
    }

    let _ = sqlx::query("PRAGMA wal_checkpoint(FULL)")
        .execute(&mut *conn)
        .await;
    let vacuum_into = format!(
        "VACUUM INTO '{}'",
        target_path.display().to_string().replace('\'', "''")
    );
    sqlx::query(&vacuum_into).execute(&mut *conn).await?;
    Ok(())
}

pub(super) async fn replace_database_file(
    database_path: &Path,
    backup_source_path: &Path,
) -> anyhow::Result<()> {
    let parent = database_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    tokio::fs::create_dir_all(&parent).await?;

    let file_name = database_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("sqlite.db");
    let temp_path = parent.join(format!("{file_name}.restore-tmp"));
    let old_path = parent.join(format!("{file_name}.restore-old"));

    let _ = tokio::fs::remove_file(&temp_path).await;
    let _ = tokio::fs::remove_file(&old_path).await;

    tokio::fs::copy(backup_source_path, &temp_path).await?;

    remove_sqlite_sidecars(database_path).await;

    let had_existing_db = tokio::fs::try_exists(database_path).await.unwrap_or(false);
    if had_existing_db {
        tokio::fs::rename(database_path, &old_path).await?;
    }

    if let Err(error) = tokio::fs::rename(&temp_path, database_path).await {
        if had_existing_db && tokio::fs::try_exists(&old_path).await.unwrap_or(false) {
            let _ = tokio::fs::rename(&old_path, database_path).await;
        }
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Err(error.into());
    }

    let _ = tokio::fs::remove_file(&old_path).await;
    remove_sqlite_sidecars(database_path).await;
    Ok(())
}

pub(super) async fn inspect_sqlite_backup(
    config: &Config,
    backup_path: &Path,
) -> anyhow::Result<SqliteBackupInspection> {
    let pool = open_readonly_sqlite_pool(backup_path).await?;
    let integrity = sqlx::query_scalar::<_, String>("PRAGMA integrity_check")
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|_| "failed".to_string());
    let tables = sqlx::query_scalar::<_, String>(
        "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default();
    let missing_tables = REQUIRED_BACKUP_TABLES
        .iter()
        .filter(|table| !tables.iter().any(|value| value == **table))
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();

    let database = DatabasePool::Sqlite(pool.clone());
    let runtime_settings = if missing_tables.iter().any(|table| table == "settings") {
        RuntimeSettings::from_defaults(config)
    } else {
        load_from_db(&database, &RuntimeSettings::from_defaults(config))
            .await
            .context("读取备份 settings 失败")?
    };

    let app_installed = if missing_tables.iter().any(|table| table == "settings") {
        false
    } else {
        sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?1")
            .bind(INSTALL_STATE_SETTING_KEY)
            .fetch_optional(&pool)
            .await?
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| matches!(value, "true" | "TRUE" | "True" | "1"))
    };

    let has_admin = if missing_tables.iter().any(|table| table == "users") {
        false
    } else {
        sqlx::query_scalar::<_, i32>("SELECT 1 FROM users WHERE id = ?1 AND role = 'admin' LIMIT 1")
            .bind(ADMIN_USER_ID)
            .fetch_optional(&pool)
            .await?
            .is_some()
    };

    pool.close().await;

    Ok(SqliteBackupInspection {
        integrity_check_passed: integrity.trim().eq_ignore_ascii_case("ok"),
        app_installed,
        has_admin,
        missing_tables,
        runtime_settings,
    })
}

pub(super) async fn load_runtime_settings_from_path(
    config: &Config,
    database_path: &Path,
) -> anyhow::Result<RuntimeSettings> {
    let pool = open_readwrite_sqlite_pool(database_path).await?;
    let database = DatabasePool::Sqlite(pool.clone());
    let settings = load_from_db(&database, &RuntimeSettings::from_defaults(config)).await?;
    pool.close().await;
    Ok(settings)
}

async fn remove_sqlite_sidecars(database_path: &Path) {
    for suffix in ["-wal", "-shm"] {
        let sidecar = PathBuf::from(format!("{}{}", database_path.display(), suffix));
        if tokio::fs::try_exists(&sidecar).await.unwrap_or(false) {
            let _ = tokio::fs::remove_file(&sidecar).await;
        }
    }
}

async fn open_readonly_sqlite_pool(path: &Path) -> anyhow::Result<sqlx::SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false)
        .read_only(true)
        .busy_timeout(Duration::from_secs(5));

    Ok(SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?)
}

async fn open_readwrite_sqlite_pool(path: &Path) -> anyhow::Result<sqlx::SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false)
        .busy_timeout(Duration::from_secs(5));

    Ok(SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?)
}
