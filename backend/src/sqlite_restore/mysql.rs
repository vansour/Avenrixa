use std::path::Path;
use std::process::Stdio;

use anyhow::Context;
use reqwest::Url;
use sqlx::Row;
use sqlx::mysql::MySqlPoolOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::config::{Config, normalize_mysql_compatible_url};
use crate::db::DatabasePool;
use crate::runtime_settings::{RuntimeSettings, load_from_db};

struct MySqlDumpTarget {
    host: String,
    port: u16,
    username: String,
    password: Option<String>,
    database: String,
}

pub(super) async fn load_runtime_settings_from_mysql(
    config: &Config,
) -> anyhow::Result<RuntimeSettings> {
    let pool = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&normalize_mysql_compatible_url(&config.database.url))
        .await?;
    let database = DatabasePool::MySql(pool.clone());
    let settings = load_from_db(&database, &RuntimeSettings::from_defaults(config)).await?;
    pool.close().await;
    Ok(settings)
}

pub(super) async fn dump_current_mysql_database_for_rollback(
    config: &Config,
    target_path: &Path,
) -> anyhow::Result<()> {
    if let Some(parent) = target_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    if tokio::fs::try_exists(target_path).await.unwrap_or(false) {
        let _ = tokio::fs::remove_file(target_path).await;
    }

    let dump_target = parse_mysql_dump_target(&config.database.url)?;
    let dump_bin = mysql_dump_binary()?;
    let mut command = tokio::process::Command::new(dump_bin);
    command
        .arg("--protocol=TCP")
        .arg(format!("--host={}", dump_target.host))
        .arg(format!("--port={}", dump_target.port))
        .arg(format!("--user={}", dump_target.username))
        .args(mysql_local_ssl_disable_args(&dump_target))
        .arg("--single-transaction")
        .arg("--skip-lock-tables")
        .arg("--no-tablespaces")
        .arg("--default-character-set=utf8mb4")
        .arg("--routines")
        .arg("--triggers")
        .arg("--events")
        .arg(format!("--result-file={}", target_path.display()))
        .arg(&dump_target.database)
        .stderr(Stdio::piped());
    if let Some(password) = dump_target.password.as_ref() {
        command.env("MYSQL_PWD", password);
    }

    let output = command.spawn()?.wait_with_output().await?;
    if !output.status.success() {
        let _ = tokio::fs::remove_file(target_path).await;
        anyhow::bail!(
            "导出 MySQL / MariaDB 回滚快照失败: {}",
            process_output_excerpt(&output.stderr).unwrap_or_else(|| output.status.to_string())
        );
    }

    let metadata = tokio::fs::metadata(target_path).await?;
    if metadata.len() == 0 {
        let _ = tokio::fs::remove_file(target_path).await;
        anyhow::bail!("导出 MySQL / MariaDB 回滚快照失败: 生成的 SQL 文件为空");
    }

    Ok(())
}

pub(super) async fn restore_mysql_dump_into_current_database(
    config: &Config,
    dump_path: &Path,
) -> anyhow::Result<()> {
    if !tokio::fs::try_exists(dump_path).await? {
        anyhow::bail!(
            "待导入的 MySQL / MariaDB 备份文件不存在: {}",
            dump_path.display()
        );
    }

    clear_mysql_schema(config).await?;

    let dump_target = parse_mysql_dump_target(&config.database.url)?;
    let client_bin = mysql_client_binary()?;
    let mut command = tokio::process::Command::new(client_bin);
    command
        .arg("--protocol=TCP")
        .arg(format!("--host={}", dump_target.host))
        .arg(format!("--port={}", dump_target.port))
        .arg(format!("--user={}", dump_target.username))
        .args(mysql_local_ssl_disable_args(&dump_target))
        .arg(format!("--database={}", dump_target.database))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if let Some(password) = dump_target.password.as_ref() {
        command.env("MYSQL_PWD", password);
    }

    let mut child = command.spawn()?;
    {
        let mut input = tokio::fs::File::open(dump_path).await?;
        let mut stdin = child.stdin.take().context("未能打开 mysql 客户端 stdin")?;
        tokio::io::copy(&mut input, &mut stdin).await?;
        stdin.shutdown().await?;
    }

    let output = child.wait_with_output().await?;
    if !output.status.success() {
        anyhow::bail!(
            "导入 MySQL / MariaDB SQL 失败: {}",
            process_output_excerpt(&output.stderr).unwrap_or_else(|| output.status.to_string())
        );
    }

    Ok(())
}

pub(super) async fn looks_like_mysql_dump(path: &Path) -> Result<bool, crate::error::AppError> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut buffer = vec![0_u8; 8192];
    let bytes_read = file.read(&mut buffer).await?;
    buffer.truncate(bytes_read);
    let header = String::from_utf8_lossy(&buffer);

    Ok(header.contains("MySQL dump")
        || header.contains("MariaDB dump")
        || header.contains("CREATE TABLE")
        || header.contains("INSERT INTO")
        || header.contains("LOCK TABLES"))
}

async fn clear_mysql_schema(config: &Config) -> anyhow::Result<()> {
    let pool = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&normalize_mysql_compatible_url(&config.database.url))
        .await?;

    sqlx::query("SET FOREIGN_KEY_CHECKS = 0")
        .execute(&pool)
        .await?;

    let views = sqlx::query_scalar::<_, String>(
        "SELECT CAST(table_name AS CHAR(255))
         FROM information_schema.views
         WHERE table_schema = DATABASE()",
    )
    .fetch_all(&pool)
    .await?;
    for view in views {
        sqlx::query(&format!(
            "DROP VIEW IF EXISTS `{}`",
            mysql_identifier(&view)
        ))
        .execute(&pool)
        .await?;
    }

    let tables = sqlx::query_scalar::<_, String>(
        "SELECT CAST(table_name AS CHAR(255))
         FROM information_schema.tables
         WHERE table_schema = DATABASE()
           AND table_type = 'BASE TABLE'",
    )
    .fetch_all(&pool)
    .await?;
    for table in tables {
        sqlx::query(&format!(
            "DROP TABLE IF EXISTS `{}`",
            mysql_identifier(&table)
        ))
        .execute(&pool)
        .await?;
    }

    let routines = sqlx::query(
        "SELECT CAST(routine_name AS CHAR(255)) AS routine_name,
                CAST(routine_type AS CHAR(32)) AS routine_type
         FROM information_schema.routines
         WHERE routine_schema = DATABASE()",
    )
    .fetch_all(&pool)
    .await?;
    for routine in routines {
        let routine_name: String = routine.try_get("routine_name")?;
        let routine_type: String = routine.try_get("routine_type")?;
        let drop_statement = if routine_type.eq_ignore_ascii_case("PROCEDURE") {
            format!(
                "DROP PROCEDURE IF EXISTS `{}`",
                mysql_identifier(&routine_name)
            )
        } else {
            format!(
                "DROP FUNCTION IF EXISTS `{}`",
                mysql_identifier(&routine_name)
            )
        };
        sqlx::query(&drop_statement).execute(&pool).await?;
    }

    let events = sqlx::query_scalar::<_, String>(
        "SELECT CAST(event_name AS CHAR(255))
         FROM information_schema.events
         WHERE event_schema = DATABASE()",
    )
    .fetch_all(&pool)
    .await?;
    for event in events {
        sqlx::query(&format!(
            "DROP EVENT IF EXISTS `{}`",
            mysql_identifier(&event)
        ))
        .execute(&pool)
        .await?;
    }

    let _ = sqlx::query("SET FOREIGN_KEY_CHECKS = 1")
        .execute(&pool)
        .await;
    pool.close().await;
    Ok(())
}

fn parse_mysql_dump_target(database_url: &str) -> anyhow::Result<MySqlDumpTarget> {
    let normalized = normalize_mysql_compatible_url(database_url);
    let url = Url::parse(&normalized)
        .map_err(|error| anyhow::anyhow!("MySQL/MariaDB 连接地址解析失败: {}", error))?;
    if url.scheme() != "mysql" {
        anyhow::bail!("MySQL / MariaDB 恢复只支持 mysql:// 或 mariadb:// 连接地址");
    }

    let host = url
        .host_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("MySQL/MariaDB 连接缺少主机地址"))?
        .to_string();
    let username = url.username().trim().to_string();
    if username.is_empty() {
        anyhow::bail!("MySQL/MariaDB 连接缺少用户名");
    }

    let database = url.path().trim_start_matches('/').trim().to_string();
    if database.is_empty() {
        anyhow::bail!("MySQL/MariaDB 连接缺少数据库名");
    }

    Ok(MySqlDumpTarget {
        host,
        port: url.port().unwrap_or(3306),
        username,
        password: url.password().map(|value| value.to_string()),
        database,
    })
}

fn mysql_dump_binary() -> anyhow::Result<String> {
    find_first_binary(&["mysqldump", "mariadb-dump"])
        .ok_or_else(|| anyhow::anyhow!("未找到 mysqldump 或 mariadb-dump"))
}

fn mysql_client_binary() -> anyhow::Result<String> {
    find_first_binary(&["mysql", "mariadb"])
        .ok_or_else(|| anyhow::anyhow!("未找到 mysql 或 mariadb 客户端"))
}

fn mysql_local_ssl_disable_args(_target: &MySqlDumpTarget) -> &'static [&'static str] {
    &[]
}

fn find_first_binary(candidates: &[&str]) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path) {
        for candidate in candidates {
            let full_path = directory.join(candidate);
            if full_path.is_file() {
                return Some(full_path.to_string_lossy().into_owned());
            }
        }
    }
    None
}

fn process_output_excerpt(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let excerpt: String = trimmed.chars().take(1000).collect();
    if trimmed.chars().count() > 1000 {
        Some(format!("{}...(truncated)", excerpt))
    } else {
        Some(excerpt)
    }
}

fn mysql_identifier(value: &str) -> String {
    value.replace('`', "``")
}
