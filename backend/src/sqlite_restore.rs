use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::Context;
use chrono::Utc;
use redis::AsyncCommands;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::info;
use uuid::Uuid;

use crate::audit::log_audit_db;
use crate::backup_manifest::{load_backup_manifest, storage_signature};
use crate::bootstrap::{resolve_sqlite_database_path, sqlite_connect_options};
use crate::config::{Config, DatabaseKind, normalize_mysql_compatible_url};
use crate::db::{ADMIN_USER_ID, DatabasePool, INSTALL_STATE_SETTING_KEY};
use crate::models::{
    BackupFileSummary, BackupObjectRollbackAnchor, BackupRestorePrecheckResponse,
    BackupRestoreResult, BackupRestoreScheduleResponse, BackupRestoreStatusResponse,
    BackupRestoreStorageSummary, PendingBackupRestore,
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

struct MySqlDumpTarget {
    host: String,
    port: u16,
    username: String,
    password: Option<String>,
    database: String,
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
    let backup_database_kind = backup_database_kind_from_filename(filename);
    let mut blockers = Vec::new();
    let mut warnings = vec![
        "恢复仅回滚数据库元数据，不会自动回滚本地图片目录或对象存储内容。".to_string(),
        "恢复计划写入后，需要立即重启服务，真正的数据库替换或导入会在下次启动前执行。".to_string(),
        "恢复成功后，所有现有登录会话都会失效，需要重新登录。".to_string(),
    ];
    let current_storage_summary = storage_summary_from_snapshot(current_storage);
    let mut integrity_check_passed = false;
    let mut app_installed = false;
    let mut has_admin = false;
    let mut storage_compatible = false;
    let mut backup_storage_summary = unknown_storage_summary();
    let mut object_rollback_anchor = None;

    match backup_database_kind {
        Some(DatabaseKind::Sqlite) => {
            if config.database.kind != DatabaseKind::Sqlite {
                blockers.push("当前实例数据库后端不是 SQLite，不能执行文件级恢复。".to_string());
            }

            if let Err(error) = resolve_sqlite_database_path(&config.database.url) {
                blockers.push(format!("当前 SQLite 连接不支持文件级恢复: {}", error));
            }

            match inspect_sqlite_backup(config, &backup_path(filename)?).await {
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
                    storage_compatible = &backup_storage == current_storage;
                    if !storage_compatible {
                        blockers.push(
                            "备份中的存储配置与当前运行配置不一致，当前恢复流程不允许自动覆盖。"
                                .to_string(),
                        );
                    }

                    integrity_check_passed = inspection.integrity_check_passed;
                    app_installed = inspection.app_installed;
                    has_admin = inspection.has_admin;
                    backup_storage_summary = storage_summary_from_snapshot(&backup_storage);
                }
                Err(error) => {
                    blockers.push(format!("无法读取并检查备份数据库: {}", error));
                }
            }

            match load_backup_manifest(filename).await {
                Ok(Some(manifest)) => {
                    object_rollback_anchor = Some(manifest.object_rollback_anchor);
                }
                Ok(None) => warnings.push(
                    "这份 SQLite 备份缺少对象/文件回滚锚点 manifest；数据库可以恢复，但附件回溯信息不完整。"
                        .to_string(),
                ),
                Err(error) => warnings.push(format!(
                    "读取备份 manifest 失败，附件回滚锚点信息不可用: {}",
                    error
                )),
            }
        }
        Some(DatabaseKind::MySql) => {
            if config.database.kind != DatabaseKind::MySql {
                blockers.push(
                    "当前实例数据库后端不是 MySQL / MariaDB，不能执行 SQL 导入恢复。".to_string(),
                );
            }

            if !looks_like_mysql_dump(&backup_path(filename)?).await? {
                blockers.push("备份文件看起来不是受支持的 MySQL SQL 导出文件。".to_string());
            } else {
                integrity_check_passed = true;
            }

            match load_backup_manifest(filename).await {
                Ok(Some(manifest)) => {
                    if !manifest.database_kind.eq_ignore_ascii_case("mysql") {
                        blockers
                            .push("备份 manifest 的数据库类型不是 MySQL / MariaDB。".to_string());
                    }
                    app_installed = manifest.app_installed;
                    has_admin = manifest.has_admin;
                    if !app_installed {
                        blockers
                            .push("备份记录显示该数据库尚未完成安装，不能用于恢复。".to_string());
                    }
                    if !has_admin {
                        blockers
                            .push("备份记录显示该数据库缺少管理员账户，不能用于恢复。".to_string());
                    }

                    storage_compatible =
                        manifest.storage_signature == storage_signature(current_storage);
                    if !storage_compatible {
                        blockers.push(
                            "备份中的存储配置与当前运行配置不一致，当前恢复流程不允许自动覆盖。"
                                .to_string(),
                        );
                    }

                    backup_storage_summary = manifest.storage;
                    object_rollback_anchor = Some(manifest.object_rollback_anchor);
                }
                Ok(None) => blockers.push(
                    "这份 MySQL / MariaDB 备份缺少恢复 manifest，无法校验存储配置与对象回滚锚点。"
                        .to_string(),
                ),
                Err(error) => blockers.push(format!(
                    "读取 MySQL / MariaDB 备份 manifest 失败: {}",
                    error
                )),
            }

            warnings.push(
                "MySQL / MariaDB 恢复会在启动前先导出 rollback_before_restore_*.mysql.sql，再清空当前 schema 并重新导入。"
                    .to_string(),
            );
        }
        Some(DatabaseKind::Postgres) => {
            blockers.push("当前版本暂不支持 PostgreSQL 备份在后台页面内直接恢复。".to_string())
        }
        None => blockers.push("备份文件名无效。".to_string()),
    }

    append_object_rollback_anchor_warnings(object_rollback_anchor.as_ref(), &mut warnings);

    Ok(BackupRestorePrecheckResponse {
        eligible: blockers.is_empty(),
        filename: backup.filename,
        backup_created_at: backup.created_at,
        backup_size_bytes: backup.size_bytes,
        current_database_kind: config.database.kind.as_str().to_string(),
        backup_database_kind: backup_database_kind
            .map(DatabaseKind::as_str)
            .unwrap_or("unknown")
            .to_string(),
        integrity_check_passed,
        app_installed,
        has_admin,
        storage_compatible,
        current_storage: current_storage_summary,
        backup_storage: backup_storage_summary,
        object_rollback_anchor,
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
            "已有待执行的 {} 恢复计划: {}，请先重启服务完成或清理它",
            restore_database_label(&existing.database_kind),
            existing.filename,
        )));
    }

    let pending = PendingBackupRestore {
        filename: filename.to_string(),
        database_kind: precheck.backup_database_kind.clone(),
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

    match backup_database_kind_from_pending(&pending) {
        Some(DatabaseKind::Sqlite) => apply_pending_sqlite_restore(config, pending).await,
        Some(DatabaseKind::MySql) => apply_pending_mysql_restore(config, pending).await,
        Some(DatabaseKind::Postgres) => {
            startup_restore_failure(
                &pending,
                Utc::now(),
                None,
                format!(
                    "待恢复计划 {} 已放弃：当前版本暂不支持 PostgreSQL 页面内恢复。",
                    pending.filename
                ),
            )
            .await
        }
        None => {
            startup_restore_failure(
                &pending,
                Utc::now(),
                None,
                format!("待恢复计划 {} 已放弃：无法识别备份类型。", pending.filename),
            )
            .await
        }
    }
}

async fn apply_pending_sqlite_restore(
    config: &Config,
    pending: PendingBackupRestore,
) -> anyhow::Result<StartupRestoreOutcome> {
    let started_at = Utc::now();
    if config.database.kind != DatabaseKind::Sqlite {
        return startup_restore_failure(
            &pending,
            started_at,
            None,
            format!(
                "待恢复计划 {} 已放弃：当前实例数据库后端不是 SQLite。",
                pending.filename
            ),
        )
        .await;
    }

    let database_path = match resolve_sqlite_database_path(&config.database.url) {
        Ok(path) => path,
        Err(error) => {
            return startup_restore_failure(
                &pending,
                started_at,
                None,
                format!(
                    "待恢复计划 {} 已放弃：当前 SQLite 地址不支持文件级恢复: {}",
                    pending.filename, error
                ),
            )
            .await;
        }
    };

    if !tokio::fs::try_exists(&database_path).await.unwrap_or(false) {
        return startup_restore_failure(
            &pending,
            started_at,
            None,
            format!(
                "待恢复计划 {} 已放弃：当前 SQLite 数据库文件不存在。",
                pending.filename
            ),
        )
        .await;
    }

    let current_settings = match load_runtime_settings_from_path(config, &database_path).await {
        Ok(settings) => settings,
        Err(error) => {
            return startup_restore_failure(
                &pending,
                started_at,
                None,
                format!(
                    "待恢复计划 {} 已放弃：读取当前数据库运行时设置失败: {}",
                    pending.filename, error
                ),
            )
            .await;
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
            return startup_restore_failure(
                &pending,
                started_at,
                None,
                format!("待恢复计划 {} 预检查失败: {}", pending.filename, error),
            )
            .await;
        }
    };
    if !precheck.eligible {
        return startup_restore_failure(
            &pending,
            started_at,
            None,
            format!(
                "待恢复计划 {} 未通过启动前校验: {}",
                pending.filename,
                precheck.blockers.join("；")
            ),
        )
        .await;
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
            startup_restore_failure(
                &pending,
                started_at,
                None,
                format!(
                    "执行 SQLite 恢复计划 {} 失败，当前数据库保持原状: {}",
                    pending.filename, error
                ),
            )
            .await
        }
    }
}

async fn apply_pending_mysql_restore(
    config: &Config,
    pending: PendingBackupRestore,
) -> anyhow::Result<StartupRestoreOutcome> {
    let started_at = Utc::now();
    if config.database.kind != DatabaseKind::MySql {
        return startup_restore_failure(
            &pending,
            started_at,
            None,
            format!(
                "待恢复计划 {} 已放弃：当前实例数据库后端不是 MySQL / MariaDB。",
                pending.filename
            ),
        )
        .await;
    }

    let current_settings = match load_runtime_settings_from_mysql(config).await {
        Ok(settings) => settings,
        Err(error) => {
            return startup_restore_failure(
                &pending,
                started_at,
                None,
                format!(
                    "待恢复计划 {} 已放弃：读取当前 MySQL / MariaDB 运行时设置失败: {}",
                    pending.filename, error
                ),
            )
            .await;
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
            return startup_restore_failure(
                &pending,
                started_at,
                None,
                format!("待恢复计划 {} 预检查失败: {}", pending.filename, error),
            )
            .await;
        }
    };
    if !precheck.eligible {
        return startup_restore_failure(
            &pending,
            started_at,
            None,
            format!(
                "待恢复计划 {} 未通过启动前校验: {}",
                pending.filename,
                precheck.blockers.join("；")
            ),
        )
        .await;
    }

    let pending_for_execution = pending.clone();
    let execution = async move {
        let rollback_filename = format!("rollback_before_restore_{}.mysql.sql", Uuid::new_v4());
        let rollback_path = backup_directory().join(&rollback_filename);
        tokio::fs::create_dir_all(backup_directory()).await?;
        dump_current_mysql_database_for_rollback(config, &rollback_path).await?;

        persist_last_restore_result(&restore_result(
            "started",
            &pending_for_execution,
            started_at,
            Some(rollback_filename.clone()),
            format!(
                "已开始在启动前恢复 MySQL / MariaDB 备份 {}，正在清空当前 schema 并导入 SQL。",
                pending_for_execution.filename
            ),
        ))
        .await?;

        clear_pending_restore_plan().await?;

        let restore_backup_path = backup_path(&pending_for_execution.filename)?;
        if let Err(error) = restore_mysql_dump_into_current_database(config, &restore_backup_path).await
        {
            let rollback_result =
                restore_mysql_dump_into_current_database(config, &rollback_path).await;
            let (status, message) = match rollback_result {
                Ok(()) => (
                    "rolled_back",
                    format!(
                        "执行 MySQL / MariaDB 恢复计划 {} 失败，已自动回滚到恢复前逻辑快照: {}",
                        pending_for_execution.filename, error
                    ),
                ),
                Err(rollback_error) => (
                    "failed",
                    format!(
                        "执行 MySQL / MariaDB 恢复计划 {} 失败，且自动回滚失败。恢复错误: {}；回滚错误: {}",
                        pending_for_execution.filename, error, rollback_error
                    ),
                ),
            };
            let result = restore_result(
                status,
                &pending_for_execution,
                started_at,
                Some(rollback_filename.clone()),
                message,
            );
            persist_last_restore_result(&result).await?;
            return Ok::<StartupRestoreOutcome, anyhow::Error>(
                StartupRestoreOutcome::StartupFailure(result),
            );
        }

        Ok::<StartupRestoreOutcome, anyhow::Error>(StartupRestoreOutcome::Applied(
            AppliedRestoreContext {
                pending: pending_for_execution,
                rollback_filename,
                started_at,
            },
        ))
    }
    .await?;

    match execution {
        StartupRestoreOutcome::Applied(context) => {
            info!(
                "MySQL/MariaDB restore import prepared from backup {}",
                context.pending.filename
            );
            Ok(StartupRestoreOutcome::Applied(context))
        }
        other => Ok(other),
    }
}

async fn startup_restore_failure(
    pending: &PendingBackupRestore,
    started_at: chrono::DateTime<chrono::Utc>,
    rollback_filename: Option<String>,
    message: String,
) -> anyhow::Result<StartupRestoreOutcome> {
    let result = restore_result("failed", pending, started_at, rollback_filename, message);
    persist_last_restore_result(&result).await?;
    clear_pending_restore_plan().await?;
    Ok(StartupRestoreOutcome::StartupFailure(result))
}

pub async fn rollback_failed_restore(
    config: &Config,
    applied: &AppliedRestoreContext,
    startup_error: &anyhow::Error,
) -> anyhow::Result<BackupRestoreResult> {
    let rollback_path = backup_directory().join(&applied.rollback_filename);

    match backup_database_kind_from_pending(&applied.pending) {
        Some(DatabaseKind::Sqlite) => {
            let database_path = resolve_sqlite_database_path(&config.database.url)?;
            replace_database_file(&database_path, &rollback_path).await?;
        }
        Some(DatabaseKind::MySql) => {
            restore_mysql_dump_into_current_database(config, &rollback_path).await?;
        }
        Some(DatabaseKind::Postgres) => {
            anyhow::bail!("PostgreSQL 恢复尚未接入自动回滚");
        }
        None => anyhow::bail!("无法识别待回滚恢复计划的数据库类型"),
    }

    let result = restore_result(
        "rolled_back",
        &applied.pending,
        applied.started_at,
        Some(applied.rollback_filename.clone()),
        format!(
            "恢复 {} 备份 {} 后启动失败，已自动回滚到恢复前快照: {}",
            restore_database_label(&applied.pending.database_kind),
            applied.pending.filename,
            startup_error
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
    let database_label = restore_database_label(&applied.pending.database_kind);
    let completion_message = if applied.pending.database_kind.eq_ignore_ascii_case("sqlite") {
        format!(
            "{} 备份 {} 已在启动前完成恢复，旧会话和缓存已全部失效。",
            database_label, applied.pending.filename
        )
    } else {
        format!(
            "{} 备份 {} 已在启动前完成导入恢复，旧会话和缓存已全部失效。",
            database_label, applied.pending.filename
        )
    };

    let result = restore_result(
        "completed",
        &applied.pending,
        applied.started_at,
        Some(applied.rollback_filename.clone()),
        completion_message,
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
            "database_kind": applied.pending.database_kind,
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
            "database_kind": result.database_kind,
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
    let action = if result.status == "rolled_back" {
        "system.database_restore.rollback_applied"
    } else {
        "system.database_restore.failed"
    };
    log_audit_db(
        &state.database,
        Some(ADMIN_USER_ID),
        action,
        "maintenance",
        None,
        None,
        Some(serde_json::json!({
            "filename": result.filename,
            "database_kind": result.database_kind,
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
        database_kind: pending.database_kind.clone(),
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

async fn load_runtime_settings_from_mysql(config: &Config) -> anyhow::Result<RuntimeSettings> {
    let pool = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&normalize_mysql_compatible_url(&config.database.url))
        .await?;
    let database = DatabasePool::MySql(pool.clone());
    let settings = load_from_db(&database, &RuntimeSettings::from_defaults(config)).await?;
    pool.close().await;
    Ok(settings)
}

async fn dump_current_mysql_database_for_rollback(
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

async fn restore_mysql_dump_into_current_database(
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

fn unknown_storage_summary() -> BackupRestoreStorageSummary {
    BackupRestoreStorageSummary {
        storage_backend: "unknown".to_string(),
        local_storage_path: String::new(),
        s3_endpoint: None,
        s3_region: None,
        s3_bucket: None,
        s3_prefix: None,
        s3_force_path_style: true,
    }
}

fn backup_database_kind_from_pending(pending: &PendingBackupRestore) -> Option<DatabaseKind> {
    DatabaseKind::parse(&pending.database_kind)
        .ok()
        .or_else(|| backup_database_kind_from_filename(&pending.filename))
}

fn backup_database_kind_from_filename(filename: &str) -> Option<DatabaseKind> {
    if filename.ends_with(".sqlite3") {
        Some(DatabaseKind::Sqlite)
    } else if filename.ends_with(".mysql.sql") {
        Some(DatabaseKind::MySql)
    } else if filename.ends_with(".sql") {
        Some(DatabaseKind::Postgres)
    } else {
        None
    }
}

fn restore_database_label(database_kind: &str) -> &'static str {
    if database_kind.eq_ignore_ascii_case("sqlite") {
        "SQLite"
    } else if database_kind.eq_ignore_ascii_case("mysql") {
        "MySQL / MariaDB"
    } else if database_kind.eq_ignore_ascii_case("postgresql")
        || database_kind.eq_ignore_ascii_case("postgres")
    {
        "PostgreSQL"
    } else {
        "数据库"
    }
}

fn append_object_rollback_anchor_warnings(
    anchor: Option<&BackupObjectRollbackAnchor>,
    warnings: &mut Vec<String>,
) {
    let Some(anchor) = anchor else {
        return;
    };

    if anchor.strategy == "local-directory-snapshot" {
        if let Some(path) = anchor.local_storage_path.as_deref() {
            warnings.push(format!(
                "这份备份绑定的文件回滚锚点目录为 {}。如需回退本地附件，请按相同时间点恢复该目录快照。",
                path
            ));
        }
        return;
    }

    if anchor.strategy == "s3-versioned-rollback-anchor" {
        let bucket = anchor
            .s3_bucket
            .clone()
            .unwrap_or_else(|| "未配置 bucket".to_string());
        let prefix = anchor.s3_prefix.clone().unwrap_or_else(|| "/".to_string());
        let status = anchor
            .s3_bucket_versioning_status
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        warnings.push(format!(
            "这份备份绑定的对象回滚锚点为 bucket={}、prefix={}、时间={}，bucket versioning 状态={status}。",
            bucket,
            prefix,
            anchor.checkpoint_at.format("%Y-%m-%d %H:%M UTC")
        ));
        warnings.push(
            "如果需要让对象存储内容与数据库版本保持一致，应按上述锚点回退对象版本，而不是单独选择任意对象快照。"
                .to_string(),
        );
        if let Some(error) = anchor.capture_error.as_ref() {
            warnings.push(format!(
                "备份生成时未能确认对象存储版本状态，锚点仍已记录，但需要额外人工核对: {}",
                error
            ));
        }
    }
}

async fn looks_like_mysql_dump(path: &Path) -> Result<bool, crate::error::AppError> {
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

fn mysql_local_ssl_disable_args(target: &MySqlDumpTarget) -> &'static [&'static str] {
    if matches!(
        target.host.as_str(),
        "mysql" | "localhost" | "127.0.0.1" | "::1"
    ) {
        &["--skip-ssl"]
    } else {
        &[]
    }
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
        && (filename.ends_with(".sqlite3")
            || filename.ends_with(".mysql.sql")
            || filename.ends_with(".sql"))
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
