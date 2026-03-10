use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use chrono::Utc;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tracing::info;
use uuid::Uuid;

use crate::audit::log_audit_db;
use crate::bootstrap::{resolve_sqlite_database_path, sqlite_connect_options};
use crate::config::{Config, DatabaseKind};
use crate::db::{ADMIN_USER_ID, DatabasePool, INSTALL_STATE_SETTING_KEY};
use crate::models::{
    BackupFileSummary, BackupRestorePrecheckResponse, BackupRestoreResult,
    BackupRestoreScheduleResponse, BackupRestoreStatusResponse, BackupRestoreStorageSummary,
    PendingBackupRestore,
};
use crate::runtime_settings::{RuntimeSettings, StorageSettingsSnapshot, load_from_db};

const BACKUP_DIR: &str = "/data/backup";
const DEFAULT_PENDING_RESTORE_PATH: &str = "/data/backup/pending_restore.json";
const DEFAULT_LAST_RESTORE_RESULT_PATH: &str = "/data/backup/last_restore_result.json";
const REQUIRED_BACKUP_TABLES: [&str; 3] = ["users", "settings", "images"];
const AUTH_VALID_AFTER_KEY: &str = "auth_valid_after";

#[derive(Debug)]
pub enum StartupRestoreOutcome {
    None,
    StartupFailure(BackupRestoreResult),
    Applied(AppliedRestoreContext),
}

#[derive(Debug, Clone)]
pub struct AppliedRestoreContext {
    pub pending: PendingBackupRestore,
    pub rollback_filename: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
struct SqliteBackupInspection {
    integrity_check_passed: bool,
    app_installed: bool,
    has_admin: bool,
    missing_tables: Vec<String>,
    runtime_settings: RuntimeSettings,
}

pub fn auth_valid_after_key() -> &'static str {
    AUTH_VALID_AFTER_KEY
}

pub fn token_issued_before_cutoff(iat: i64, valid_after: Option<i64>) -> bool {
    valid_after.is_some_and(|cutoff| iat < cutoff)
}

pub async fn load_restore_status() -> anyhow::Result<BackupRestoreStatusResponse> {
    Ok(BackupRestoreStatusResponse {
        pending: load_pending_restore_plan().await?,
        last_result: load_last_restore_result().await?,
    })
}

pub async fn precheck_restore(
    config: &Config,
    current_storage: &StorageSettingsSnapshot,
    filename: &str,
) -> Result<BackupRestorePrecheckResponse, crate::error::AppError> {
    let backup = backup_file_summary(filename).await?;
    let mut blockers = Vec::new();
    let mut warnings = vec![
        "恢复仅回滚数据库元数据，不会回滚本地图片文件或 S3 对象。".to_string(),
        "恢复计划写入后，需要立即重启服务，真正的数据替换会在下次启动前执行。".to_string(),
        "恢复成功后，所有现有登录会话都会失效，需要重新登录。".to_string(),
    ];

    if config.database.kind != DatabaseKind::Sqlite {
        blockers.push("当前实例数据库后端不是 SQLite，不能执行文件级恢复。".to_string());
    }

    if let Err(error) = resolve_sqlite_database_path(&config.database.url) {
        blockers.push(format!("当前 SQLite 连接不支持文件级恢复: {}", error));
    }

    let current_storage_summary = storage_summary_from_snapshot(current_storage);

    let (
        integrity_check_passed,
        app_installed,
        has_admin,
        storage_compatible,
        backup_storage_summary,
    ) = match inspect_sqlite_backup(config, &backup_path(filename)?).await {
        Ok(inspection) => {
            for table in &inspection.missing_tables {
                blockers.push(format!("备份缺少核心表: {}", table));
            }
            if !inspection.integrity_check_passed {
                blockers.push("备份文件未通过 SQLite integrity_check。".to_string());
            }
            if !inspection.app_installed {
                blockers.push("备份数据库尚未完成安装，不能用于恢复。".to_string());
            }
            if !inspection.has_admin {
                blockers.push("备份数据库不存在管理员账户，不能用于恢复。".to_string());
            }

            let backup_storage = inspection.runtime_settings.storage_settings();
            let storage_compatible = &backup_storage == current_storage;
            if !storage_compatible {
                blockers.push(
                    "备份中的存储配置与当前运行配置不一致，第一版恢复流程不允许自动覆盖。"
                        .to_string(),
                );
                warnings.push("如需恢复到不同存储配置，后续需要单独实现强制恢复流程。".to_string());
            }

            (
                inspection.integrity_check_passed,
                inspection.app_installed,
                inspection.has_admin,
                storage_compatible,
                storage_summary_from_snapshot(&backup_storage),
            )
        }
        Err(error) => {
            blockers.push(format!("无法读取并检查备份数据库: {}", error));
            (
                false,
                false,
                false,
                false,
                BackupRestoreStorageSummary {
                    storage_backend: "unknown".to_string(),
                    local_storage_path: String::new(),
                    s3_endpoint: None,
                    s3_region: None,
                    s3_bucket: None,
                    s3_prefix: None,
                    s3_force_path_style: true,
                },
            )
        }
    };

    Ok(BackupRestorePrecheckResponse {
        eligible: blockers.is_empty(),
        filename: backup.filename,
        backup_created_at: backup.created_at,
        backup_size_bytes: backup.size_bytes,
        current_database_kind: config.database.kind.as_str().to_string(),
        integrity_check_passed,
        app_installed,
        has_admin,
        storage_compatible,
        current_storage: current_storage_summary,
        backup_storage: backup_storage_summary,
        warnings,
        blockers,
    })
}

pub async fn schedule_restore(
    config: &Config,
    current_storage: &StorageSettingsSnapshot,
    requested_by_user_id: Uuid,
    requested_by_email: &str,
    filename: &str,
) -> Result<BackupRestoreScheduleResponse, crate::error::AppError> {
    let precheck = precheck_restore(config, current_storage, filename).await?;
    if !precheck.eligible {
        return Err(crate::error::AppError::ValidationError(
            precheck.blockers.join("；"),
        ));
    }

    if let Some(existing) = load_pending_restore_plan()
        .await
        .map_err(|error| crate::error::AppError::Internal(error.into()))?
    {
        if existing.filename == filename {
            return Ok(BackupRestoreScheduleResponse {
                scheduled: true,
                restart_required: true,
                pending: existing,
                precheck,
            });
        }

        return Err(crate::error::AppError::ValidationError(format!(
            "已有待执行的 SQLite 恢复计划: {}，请先重启服务完成或清理它",
            existing.filename
        )));
    }

    let pending = PendingBackupRestore {
        filename: filename.to_string(),
        requested_by_user_id,
        requested_by_email: requested_by_email.to_string(),
        scheduled_at: Utc::now(),
        backup_created_at: precheck.backup_created_at,
        backup_size_bytes: precheck.backup_size_bytes,
    };

    write_json_file(&pending_restore_path(), &pending)
        .await
        .map_err(|error| crate::error::AppError::Internal(error.into()))?;

    Ok(BackupRestoreScheduleResponse {
        scheduled: true,
        restart_required: true,
        pending,
        precheck,
    })
}

pub async fn apply_pending_restore_if_any(
    config: &Config,
) -> anyhow::Result<StartupRestoreOutcome> {
    let Some(pending) = load_pending_restore_plan().await? else {
        return Ok(StartupRestoreOutcome::None);
    };

    let started_at = Utc::now();
    if config.database.kind != DatabaseKind::Sqlite {
        let result = restore_result(
            "failed",
            &pending,
            started_at,
            None,
            format!(
                "待恢复计划 {} 已放弃：当前实例数据库后端不是 SQLite。",
                pending.filename
            ),
        );
        persist_last_restore_result(&result).await?;
        clear_pending_restore_plan().await?;
        return Ok(StartupRestoreOutcome::StartupFailure(result));
    }

    let database_path = match resolve_sqlite_database_path(&config.database.url) {
        Ok(path) => path,
        Err(error) => {
            let result = restore_result(
                "failed",
                &pending,
                started_at,
                None,
                format!(
                    "待恢复计划 {} 已放弃：当前 SQLite 地址不支持文件级恢复: {}",
                    pending.filename, error
                ),
            );
            persist_last_restore_result(&result).await?;
            clear_pending_restore_plan().await?;
            return Ok(StartupRestoreOutcome::StartupFailure(result));
        }
    };

    if !tokio::fs::try_exists(&database_path).await.unwrap_or(false) {
        let result = restore_result(
            "failed",
            &pending,
            started_at,
            None,
            format!(
                "待恢复计划 {} 已放弃：当前 SQLite 数据库文件不存在。",
                pending.filename
            ),
        );
        persist_last_restore_result(&result).await?;
        clear_pending_restore_plan().await?;
        return Ok(StartupRestoreOutcome::StartupFailure(result));
    }

    let current_settings = match load_runtime_settings_from_path(config, &database_path).await {
        Ok(settings) => settings,
        Err(error) => {
            let result = restore_result(
                "failed",
                &pending,
                started_at,
                None,
                format!(
                    "待恢复计划 {} 已放弃：读取当前数据库运行时设置失败: {}",
                    pending.filename, error
                ),
            );
            persist_last_restore_result(&result).await?;
            clear_pending_restore_plan().await?;
            return Ok(StartupRestoreOutcome::StartupFailure(result));
        }
    };
    let precheck = match precheck_restore(
        config,
        &current_settings.storage_settings(),
        &pending.filename,
    )
    .await
    {
        Ok(precheck) => precheck,
        Err(error) => {
            let result = restore_result(
                "failed",
                &pending,
                started_at,
                None,
                format!("待恢复计划 {} 预检查失败: {}", pending.filename, error),
            );
            persist_last_restore_result(&result).await?;
            clear_pending_restore_plan().await?;
            return Ok(StartupRestoreOutcome::StartupFailure(result));
        }
    };
    if !precheck.eligible {
        let result = restore_result(
            "failed",
            &pending,
            started_at,
            None,
            format!(
                "待恢复计划 {} 未通过启动前校验: {}",
                pending.filename,
                precheck.blockers.join("；")
            ),
        );
        persist_last_restore_result(&result).await?;
        clear_pending_restore_plan().await?;
        return Ok(StartupRestoreOutcome::StartupFailure(result));
    }

    let pending_for_execution = pending.clone();
    let execution = async move {
        let rollback_filename = format!("rollback_before_restore_{}.sqlite3", Uuid::new_v4());
        let rollback_path = backup_directory().join(&rollback_filename);
        tokio::fs::create_dir_all(backup_directory()).await?;
        create_sqlite_snapshot(config, &rollback_path).await?;

        persist_last_restore_result(&restore_result(
            "started",
            &pending_for_execution,
            started_at,
            Some(rollback_filename.clone()),
            format!(
                "已开始在启动前恢复 SQLite 备份 {}，正在执行文件替换。",
                pending_for_execution.filename
            ),
        ))
        .await?;

        clear_pending_restore_plan().await?;
        replace_database_file(
            &database_path,
            &backup_path(&pending_for_execution.filename)?,
        )
        .await?;

        Ok::<AppliedRestoreContext, anyhow::Error>(AppliedRestoreContext {
            pending: pending_for_execution,
            rollback_filename,
            started_at,
        })
    }
    .await;

    match execution {
        Ok(context) => {
            info!(
                "SQLite restore file swap prepared from backup {}",
                context.pending.filename
            );
            Ok(StartupRestoreOutcome::Applied(context))
        }
        Err(error) => {
            let result = restore_result(
                "failed",
                &pending,
                started_at,
                None,
                format!(
                    "执行 SQLite 恢复计划 {} 失败，当前数据库保持原状: {}",
                    pending.filename, error
                ),
            );
            persist_last_restore_result(&result).await?;
            let _ = clear_pending_restore_plan().await;
            Ok(StartupRestoreOutcome::StartupFailure(result))
        }
    }
}

pub async fn rollback_failed_restore(
    config: &Config,
    applied: &AppliedRestoreContext,
    startup_error: &anyhow::Error,
) -> anyhow::Result<BackupRestoreResult> {
    let database_path = resolve_sqlite_database_path(&config.database.url)?;
    let rollback_path = backup_directory().join(&applied.rollback_filename);

    replace_database_file(&database_path, &rollback_path).await?;

    let result = restore_result(
        "rolled_back",
        &applied.pending,
        applied.started_at,
        Some(applied.rollback_filename.clone()),
        format!(
            "恢复备份 {} 后启动失败，已自动回滚到恢复前快照: {}",
            applied.pending.filename, startup_error
        ),
    );
    persist_last_restore_result(&result).await?;
    Ok(result)
}

pub async fn finalize_restore_success(
    state: &crate::db::AppState,
    applied: &AppliedRestoreContext,
) -> anyhow::Result<BackupRestoreResult> {
    invalidate_redis_after_restore(state).await?;

    let result = restore_result(
        "completed",
        &applied.pending,
        applied.started_at,
        Some(applied.rollback_filename.clone()),
        format!(
            "SQLite 备份 {} 已在启动前完成恢复，旧会话和缓存已全部失效。",
            applied.pending.filename
        ),
    );
    persist_last_restore_result(&result).await?;

    log_audit_db(
        &state.database,
        Some(ADMIN_USER_ID),
        "system.database_restore.completed",
        "maintenance",
        None,
        None,
        Some(serde_json::json!({
            "filename": applied.pending.filename,
            "requested_by_email": applied.pending.requested_by_email,
            "scheduled_at": applied.pending.scheduled_at,
            "rollback_filename": applied.rollback_filename,
            "result": "completed",
            "risk_level": "danger",
        })),
    )
    .await;

    Ok(result)
}

pub async fn finalize_restore_rollback(
    state: &crate::db::AppState,
    result: &BackupRestoreResult,
) -> anyhow::Result<()> {
    log_audit_db(
        &state.database,
        Some(ADMIN_USER_ID),
        "system.database_restore.rollback_applied",
        "maintenance",
        None,
        None,
        Some(serde_json::json!({
            "filename": result.filename,
            "rollback_filename": result.rollback_filename,
            "message": result.message,
            "result": "rolled_back",
            "risk_level": "danger",
        })),
    )
    .await;
    Ok(())
}

pub async fn record_startup_restore_failure(
    state: &crate::db::AppState,
    result: &BackupRestoreResult,
) -> anyhow::Result<()> {
    log_audit_db(
        &state.database,
        Some(ADMIN_USER_ID),
        "system.database_restore.failed",
        "maintenance",
        None,
        None,
        Some(serde_json::json!({
            "filename": result.filename,
            "message": result.message,
            "rollback_filename": result.rollback_filename,
            "result": result.status,
            "risk_level": "danger",
        })),
    )
    .await;
    Ok(())
}

fn restore_result(
    status: &str,
    pending: &PendingBackupRestore,
    started_at: chrono::DateTime<chrono::Utc>,
    rollback_filename: Option<String>,
    message: String,
) -> BackupRestoreResult {
    BackupRestoreResult {
        status: status.to_string(),
        filename: pending.filename.clone(),
        message,
        scheduled_at: Some(pending.scheduled_at),
        started_at: Some(started_at),
        finished_at: Utc::now(),
        rollback_filename,
    }
}

async fn invalidate_redis_after_restore(state: &crate::db::AppState) -> anyhow::Result<()> {
    let mut redis = state.redis.clone();
    let cutoff = Utc::now().timestamp();
    let _: () = redis.set(auth_valid_after_key(), cutoff).await?;

    for pattern in [
        "token_revoked:*",
        "user_token_version:*",
        "images:list:*",
        "hash:*",
        "hash:info:*",
        "img:*",
    ] {
        let _ = crate::cache::Cache::del_pattern(&mut redis, pattern).await;
    }

    Ok(())
}

async fn create_sqlite_snapshot(config: &Config, target_path: &Path) -> anyhow::Result<()> {
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

async fn replace_database_file(
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

async fn remove_sqlite_sidecars(database_path: &Path) {
    for suffix in ["-wal", "-shm"] {
        let sidecar = PathBuf::from(format!("{}{}", database_path.display(), suffix));
        if tokio::fs::try_exists(&sidecar).await.unwrap_or(false) {
            let _ = tokio::fs::remove_file(&sidecar).await;
        }
    }
}

async fn inspect_sqlite_backup(
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

async fn load_runtime_settings_from_path(
    config: &Config,
    database_path: &Path,
) -> anyhow::Result<RuntimeSettings> {
    let pool = open_readwrite_sqlite_pool(database_path).await?;
    let database = DatabasePool::Sqlite(pool.clone());
    let settings = load_from_db(&database, &RuntimeSettings::from_defaults(config)).await?;
    pool.close().await;
    Ok(settings)
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

fn storage_summary_from_snapshot(
    snapshot: &StorageSettingsSnapshot,
) -> BackupRestoreStorageSummary {
    BackupRestoreStorageSummary {
        storage_backend: snapshot.storage_backend.as_str().to_string(),
        local_storage_path: snapshot.local_storage_path.clone(),
        s3_endpoint: snapshot.s3_endpoint.clone(),
        s3_region: snapshot.s3_region.clone(),
        s3_bucket: snapshot.s3_bucket.clone(),
        s3_prefix: snapshot.s3_prefix.clone(),
        s3_force_path_style: snapshot.s3_force_path_style,
    }
}

async fn load_pending_restore_plan() -> anyhow::Result<Option<PendingBackupRestore>> {
    read_json_file(&pending_restore_path()).await
}

async fn load_last_restore_result() -> anyhow::Result<Option<BackupRestoreResult>> {
    read_json_file(&last_restore_result_path()).await
}

async fn persist_last_restore_result(result: &BackupRestoreResult) -> anyhow::Result<()> {
    write_json_file(&last_restore_result_path(), result).await
}

async fn clear_pending_restore_plan() -> anyhow::Result<()> {
    let path = pending_restore_path();
    if tokio::fs::try_exists(&path).await.unwrap_or(false) {
        tokio::fs::remove_file(path).await?;
    }
    Ok(())
}

async fn read_json_file<T>(path: &Path) -> anyhow::Result<Option<T>>
where
    T: for<'de> Deserialize<'de>,
{
    if !tokio::fs::try_exists(path).await? {
        return Ok(None);
    }
    let content = tokio::fs::read_to_string(path).await?;
    let parsed = serde_json::from_str::<T>(&content)?;
    Ok(Some(parsed))
}

async fn write_json_file<T>(path: &Path, value: &T) -> anyhow::Result<()>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let content = serde_json::to_string_pretty(value)?;
    tokio::fs::write(path, content).await?;
    Ok(())
}

fn pending_restore_path() -> PathBuf {
    std::env::var("SQLITE_PENDING_RESTORE_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PENDING_RESTORE_PATH))
}

fn last_restore_result_path() -> PathBuf {
    std::env::var("SQLITE_LAST_RESTORE_RESULT_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LAST_RESTORE_RESULT_PATH))
}

fn backup_directory() -> PathBuf {
    PathBuf::from(BACKUP_DIR)
}

fn validate_backup_filename(filename: &str) -> bool {
    !filename.is_empty()
        && filename.len() <= 255
        && filename.starts_with("backup_")
        && filename.ends_with(".sqlite3")
        && filename.bytes().all(|byte| {
            matches!(
                byte,
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'_' | b'-'
            )
        })
}

fn backup_path(filename: &str) -> Result<PathBuf, crate::error::AppError> {
    if !validate_backup_filename(filename) {
        return Err(crate::error::AppError::ValidationError(
            "备份文件名无效".to_string(),
        ));
    }

    Ok(backup_directory().join(filename))
}

async fn backup_file_summary(filename: &str) -> Result<BackupFileSummary, crate::error::AppError> {
    let path = backup_path(filename)?;
    let metadata = match tokio::fs::metadata(&path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(crate::error::AppError::BackupNotFound);
        }
        Err(error) => return Err(crate::error::AppError::IoError(error)),
    };

    Ok(BackupFileSummary {
        filename: filename.to_string(),
        created_at: metadata
            .modified()
            .or_else(|_| metadata.created())
            .ok()
            .map(chrono::DateTime::<chrono::Utc>::from)
            .unwrap_or_else(Utc::now),
        size_bytes: metadata.len(),
    })
}
