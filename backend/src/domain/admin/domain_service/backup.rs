use chrono::{DateTime, Utc};
use reqwest::Url;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tracing::{error, info, warn};
use uuid::Uuid;

use super::AdminDomainService;
use crate::audit::log_audit_db;
use crate::backup_manifest::{backup_directory, capture_backup_manifest, write_backup_manifest};
use crate::config::{DatabaseKind, normalize_mysql_compatible_url};
use crate::db::DatabasePool;
use crate::error::AppError;
use crate::models::{
    BackupFileSummary, BackupResponse, BackupSemantics, backup_semantics_from_database_kind,
    infer_backup_semantics,
};
use crate::runtime_settings::StorageSettingsSnapshot;

struct MySqlDumpTarget {
    host: String,
    port: u16,
    username: String,
    password: Option<String>,
    database: String,
}

const DEFAULT_BACKUP_COMMAND_TIMEOUT: Duration = Duration::from_secs(30 * 60);

struct ExternalCommandOutcome {
    stderr_excerpt: Option<String>,
}

fn spawn_backup_audit(
    database: DatabasePool,
    admin_user_id: Uuid,
    action: &'static str,
    details: serde_json::Value,
) {
    tokio::spawn(async move {
        log_audit_db(
            &database,
            Some(admin_user_id),
            action,
            "maintenance",
            None,
            None,
            Some(details),
        )
        .await;
    });
}

impl AdminDomainService {
    pub async fn list_backups(&self) -> Result<Vec<BackupFileSummary>, AppError> {
        let backup_dir = backup_directory();
        if !tokio::fs::try_exists(&backup_dir).await.unwrap_or(false) {
            return Ok(Vec::new());
        }

        let mut directory = tokio::fs::read_dir(&backup_dir).await?;
        let mut backups = Vec::new();

        while let Some(entry) = directory.next_entry().await? {
            let Ok(file_type) = entry.file_type().await else {
                continue;
            };
            if !file_type.is_file() {
                continue;
            }

            let Some(filename) = entry.file_name().to_str().map(ToOwned::to_owned) else {
                continue;
            };
            if !is_valid_backup_filename(&filename) {
                continue;
            }

            let Ok(metadata) = entry.metadata().await else {
                continue;
            };
            let created_at = file_timestamp(&metadata).unwrap_or_else(Utc::now);
            let semantics = infer_backup_semantics(&filename, None);

            backups.push(BackupFileSummary {
                filename,
                created_at,
                size_bytes: metadata.len(),
                semantics,
            });
        }

        backups.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.filename.cmp(&left.filename))
        });

        Ok(backups)
    }

    pub async fn backup_database(
        &self,
        admin_user_id: Uuid,
        admin_email: &str,
    ) -> Result<BackupResponse, AppError> {
        tokio::fs::create_dir_all(backup_directory()).await?;
        let storage_settings = self.storage_manager.active_settings().storage_settings();

        match &self.database {
            DatabasePool::Postgres(_) => {
                let semantics = backup_semantics_from_database_kind(DatabaseKind::Postgres);
                let filename = format!("backup_{}.sql", Uuid::new_v4());
                let backup_path = backup_path(&filename)?;
                let database = self.database.clone();
                let database_url = &self.config.database.url;
                let dump_bin = pg_dump_binary().map_err(AppError::Internal)?;
                let mut command = Command::new(&dump_bin);
                command
                    .arg("--dbname")
                    .arg(database_url)
                    .arg("--format=plain")
                    .arg("--no-owner")
                    .arg("--no-acl")
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());

                let warning_excerpt =
                    match run_streaming_dump_command("pg_dump", command, &backup_path).await {
                        Ok(outcome) => outcome.stderr_excerpt,
                        Err(error) => {
                            error!("pg_dump backup failed: {}", error);
                            spawn_backup_audit(
                                database,
                                admin_user_id,
                                "admin.maintenance.database_backup.failed",
                                serde_json::json!({
                                    "admin_email": admin_email,
                                    "error": error.to_string(),
                                    "result": "failed",
                                    "risk_level": "info",
                                    "database_kind": "postgresql",
                                }),
                            );
                            return Err(AppError::Internal(error));
                        }
                    };

                let backup_size_bytes = match ensure_nonempty_backup_file(&backup_path).await {
                    Ok(size) => size,
                    Err(error) => {
                        let _ = tokio::fs::remove_file(&backup_path).await;
                        spawn_backup_audit(
                            self.database.clone(),
                            admin_user_id,
                            "admin.maintenance.database_backup.failed",
                            serde_json::json!({
                                "admin_email": admin_email,
                                "filename": filename,
                                "error": error.to_string(),
                                "result": "failed",
                                "risk_level": "info",
                                "database_kind": "postgresql",
                            }),
                        );
                        return Err(AppError::Internal(error));
                    }
                };

                let created_at = Utc::now();
                if let Err(error) = persist_backup_manifest(
                    &filename,
                    DatabaseKind::Postgres,
                    semantics.clone(),
                    created_at,
                    &storage_settings,
                )
                .await
                {
                    let _ = tokio::fs::remove_file(&backup_path).await;
                    spawn_backup_audit(
                        self.database.clone(),
                        admin_user_id,
                        "admin.maintenance.database_backup.failed",
                        serde_json::json!({
                            "admin_email": admin_email,
                            "filename": filename,
                            "error": error.to_string(),
                            "result": "failed",
                            "risk_level": "info",
                            "database_kind": "postgresql",
                        }),
                    );
                    return Err(AppError::Internal(error));
                }

                info!("Database backup created: {} by {}", filename, admin_email);
                spawn_backup_audit(
                    self.database.clone(),
                    admin_user_id,
                    "admin.maintenance.database_backup.created",
                    serde_json::json!({
                        "admin_email": admin_email,
                        "filename": filename,
                        "result": "completed",
                        "risk_level": "info",
                        "database_kind": "postgresql",
                        "backup_kind": semantics.backup_kind.clone(),
                        "backup_scope": semantics.backup_scope.clone(),
                        "restore_mode": semantics.restore_mode.clone(),
                        "ui_restore_supported": semantics.ui_restore_supported,
                        "backup_size_bytes": backup_size_bytes,
                        "warning_excerpt": warning_excerpt,
                    }),
                );

                Ok(BackupResponse {
                    filename,
                    created_at,
                    semantics,
                })
            }
            DatabasePool::MySql(_) => {
                let semantics = backup_semantics_from_database_kind(DatabaseKind::MySql);
                let filename = format!("backup_{}.mysql.sql", Uuid::new_v4());
                let backup_path = backup_path(&filename)?;
                let database = self.database.clone();
                let dump_target = parse_mysql_dump_target(&self.config.database.url)
                    .map_err(AppError::Internal)?;
                let dump_bin = mysql_dump_binary().map_err(AppError::Internal)?;

                if tokio::fs::try_exists(&backup_path).await.unwrap_or(false) {
                    let _ = tokio::fs::remove_file(&backup_path).await;
                }

                let mut command = Command::new(&dump_bin);
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
                    .arg(format!("--result-file={}", backup_path.display()))
                    .arg(&dump_target.database)
                    .stdout(Stdio::null())
                    .stderr(Stdio::piped());
                if let Some(password) = dump_target.password.as_ref() {
                    command.env("MYSQL_PWD", password);
                }

                let warning_excerpt =
                    match run_dump_command_with_result_file("mysqldump", command, &backup_path)
                        .await
                    {
                        Ok(outcome) => outcome.stderr_excerpt,
                        Err(error) => {
                            error!("mysqldump backup failed: {}", error);
                            spawn_backup_audit(
                                database,
                                admin_user_id,
                                "admin.maintenance.database_backup.failed",
                                serde_json::json!({
                                    "admin_email": admin_email,
                                    "error": error.to_string(),
                                    "result": "failed",
                                    "risk_level": "info",
                                    "database_kind": "mysql",
                                }),
                            );
                            return Err(AppError::Internal(error));
                        }
                    };

                let backup_size_bytes = match ensure_nonempty_backup_file(&backup_path).await {
                    Ok(size) => size,
                    Err(error) => {
                        let _ = tokio::fs::remove_file(&backup_path).await;
                        spawn_backup_audit(
                            self.database.clone(),
                            admin_user_id,
                            "admin.maintenance.database_backup.failed",
                            serde_json::json!({
                                "admin_email": admin_email,
                                "filename": filename,
                                "error": error.to_string(),
                                "result": "failed",
                                "risk_level": "info",
                                "database_kind": "mysql",
                            }),
                        );
                        return Err(AppError::Internal(error));
                    }
                };

                let created_at = Utc::now();
                if let Err(error) = persist_backup_manifest(
                    &filename,
                    DatabaseKind::MySql,
                    semantics.clone(),
                    created_at,
                    &storage_settings,
                )
                .await
                {
                    let _ = tokio::fs::remove_file(&backup_path).await;
                    spawn_backup_audit(
                        self.database.clone(),
                        admin_user_id,
                        "admin.maintenance.database_backup.failed",
                        serde_json::json!({
                            "admin_email": admin_email,
                            "filename": filename,
                            "error": error.to_string(),
                            "result": "failed",
                            "risk_level": "info",
                            "database_kind": "mysql",
                        }),
                    );
                    return Err(AppError::Internal(error));
                }

                info!(
                    "MySQL/MariaDB backup created: {} by {}",
                    filename, admin_email
                );
                spawn_backup_audit(
                    self.database.clone(),
                    admin_user_id,
                    "admin.maintenance.database_backup.created",
                    serde_json::json!({
                        "admin_email": admin_email,
                        "filename": filename,
                        "result": "completed",
                        "risk_level": "info",
                        "database_kind": "mysql",
                        "backup_kind": semantics.backup_kind.clone(),
                        "backup_scope": semantics.backup_scope.clone(),
                        "restore_mode": semantics.restore_mode.clone(),
                        "ui_restore_supported": semantics.ui_restore_supported,
                        "backup_size_bytes": backup_size_bytes,
                        "warning_excerpt": warning_excerpt,
                    }),
                );

                Ok(BackupResponse {
                    filename,
                    created_at,
                    semantics,
                })
            }
            DatabasePool::Sqlite(pool) => {
                let semantics = backup_semantics_from_database_kind(DatabaseKind::Sqlite);
                let filename = format!("backup_{}.sqlite3", Uuid::new_v4());
                let backup_path = backup_path(&filename)?;
                let backup_sql = format!(
                    "VACUUM INTO '{}'",
                    backup_path.display().to_string().replace('\'', "''")
                );
                let mut conn = pool.acquire().await?;

                if tokio::fs::try_exists(&backup_path).await.unwrap_or(false) {
                    let _ = tokio::fs::remove_file(&backup_path).await;
                }

                // Flush WAL before exporting a compact backup file.
                let _ = sqlx::query("PRAGMA wal_checkpoint(FULL)")
                    .execute(&mut *conn)
                    .await;
                let result = sqlx::query(&backup_sql).execute(&mut *conn).await;
                if let Err(error) = result {
                    error!("SQLite backup failed: {}", error);
                    let _ = tokio::fs::remove_file(&backup_path).await;
                    spawn_backup_audit(
                        self.database.clone(),
                        admin_user_id,
                        "admin.maintenance.database_backup.failed",
                        serde_json::json!({
                            "admin_email": admin_email,
                            "error": error.to_string(),
                            "result": "failed",
                            "risk_level": "info",
                            "database_kind": "sqlite",
                        }),
                    );
                    return Err(AppError::Internal(anyhow::anyhow!(
                        "SQLite backup failed: {}",
                        error
                    )));
                }

                let backup_size_bytes = match ensure_nonempty_backup_file(&backup_path).await {
                    Ok(size) => size,
                    Err(error) => {
                        let _ = tokio::fs::remove_file(&backup_path).await;
                        spawn_backup_audit(
                            self.database.clone(),
                            admin_user_id,
                            "admin.maintenance.database_backup.failed",
                            serde_json::json!({
                                "admin_email": admin_email,
                                "filename": filename,
                                "error": error.to_string(),
                                "result": "failed",
                                "risk_level": "info",
                                "database_kind": "sqlite",
                            }),
                        );
                        return Err(AppError::Internal(error));
                    }
                };

                let created_at = Utc::now();
                if let Err(error) = persist_backup_manifest(
                    &filename,
                    DatabaseKind::Sqlite,
                    semantics.clone(),
                    created_at,
                    &storage_settings,
                )
                .await
                {
                    let _ = tokio::fs::remove_file(&backup_path).await;
                    spawn_backup_audit(
                        self.database.clone(),
                        admin_user_id,
                        "admin.maintenance.database_backup.failed",
                        serde_json::json!({
                            "admin_email": admin_email,
                            "filename": filename,
                            "error": error.to_string(),
                            "result": "failed",
                            "risk_level": "info",
                            "database_kind": "sqlite",
                        }),
                    );
                    return Err(AppError::Internal(error));
                }

                info!("SQLite backup created: {} by {}", filename, admin_email);
                spawn_backup_audit(
                    self.database.clone(),
                    admin_user_id,
                    "admin.maintenance.database_backup.created",
                    serde_json::json!({
                        "admin_email": admin_email,
                        "filename": filename,
                        "result": "completed",
                        "risk_level": "info",
                        "database_kind": "sqlite",
                        "backup_kind": semantics.backup_kind.clone(),
                        "backup_scope": semantics.backup_scope.clone(),
                        "restore_mode": semantics.restore_mode.clone(),
                        "ui_restore_supported": semantics.ui_restore_supported,
                        "backup_size_bytes": backup_size_bytes,
                    }),
                );

                Ok(BackupResponse {
                    filename,
                    created_at,
                    semantics,
                })
            }
        }
    }

    pub async fn download_backup(
        &self,
        admin_user_id: Uuid,
        admin_email: &str,
        filename: &str,
    ) -> Result<Vec<u8>, AppError> {
        let backup_path = backup_path(filename)?;
        let bytes = match tokio::fs::read(&backup_path).await {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(AppError::BackupNotFound);
            }
            Err(error) => return Err(AppError::IoError(error)),
        };

        spawn_backup_audit(
            self.database.clone(),
            admin_user_id,
            "admin.maintenance.database_backup.downloaded",
            serde_json::json!({
                "admin_email": admin_email,
                "filename": filename,
                "result": "completed",
                "risk_level": "info",
                "size_bytes": bytes.len(),
            }),
        );

        Ok(bytes)
    }

    pub async fn delete_backup(
        &self,
        admin_user_id: Uuid,
        admin_email: &str,
        filename: &str,
    ) -> Result<(), AppError> {
        let backup_path = backup_path(filename)?;
        let size_bytes = match tokio::fs::metadata(&backup_path).await {
            Ok(metadata) => metadata.len(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                spawn_backup_audit(
                    self.database.clone(),
                    admin_user_id,
                    "admin.maintenance.database_backup.delete_failed",
                    serde_json::json!({
                        "admin_email": admin_email,
                        "filename": filename,
                        "error": "backup file not found",
                        "result": "failed",
                        "risk_level": "danger",
                    }),
                );
                return Err(AppError::BackupNotFound);
            }
            Err(error) => {
                spawn_backup_audit(
                    self.database.clone(),
                    admin_user_id,
                    "admin.maintenance.database_backup.delete_failed",
                    serde_json::json!({
                        "admin_email": admin_email,
                        "filename": filename,
                        "error": error.to_string(),
                        "result": "failed",
                        "risk_level": "danger",
                    }),
                );
                return Err(AppError::IoError(error));
            }
        };

        match tokio::fs::remove_file(&backup_path).await {
            Ok(()) => {
                spawn_backup_audit(
                    self.database.clone(),
                    admin_user_id,
                    "admin.maintenance.database_backup.deleted",
                    serde_json::json!({
                        "admin_email": admin_email,
                        "filename": filename,
                        "size_bytes": size_bytes,
                        "result": "completed",
                        "risk_level": "danger",
                    }),
                );
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                spawn_backup_audit(
                    self.database.clone(),
                    admin_user_id,
                    "admin.maintenance.database_backup.delete_failed",
                    serde_json::json!({
                        "admin_email": admin_email,
                        "filename": filename,
                        "error": "backup file not found",
                        "result": "failed",
                        "risk_level": "danger",
                    }),
                );
                Err(AppError::BackupNotFound)
            }
            Err(error) => {
                spawn_backup_audit(
                    self.database.clone(),
                    admin_user_id,
                    "admin.maintenance.database_backup.delete_failed",
                    serde_json::json!({
                        "admin_email": admin_email,
                        "filename": filename,
                        "error": error.to_string(),
                        "result": "failed",
                        "risk_level": "danger",
                    }),
                );
                Err(AppError::IoError(error))
            }
        }
    }
}

fn is_valid_backup_filename(filename: &str) -> bool {
    !filename.is_empty()
        && filename.len() <= 255
        && filename.starts_with("backup_")
        && (filename.ends_with(".sql")
            || filename.ends_with(".mysql.sql")
            || filename.ends_with(".sqlite3"))
        && filename.bytes().all(|byte| {
            matches!(
                byte,
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'_' | b'-'
            )
        })
}

async fn persist_backup_manifest(
    filename: &str,
    database_kind: DatabaseKind,
    semantics: BackupSemantics,
    created_at: DateTime<Utc>,
    storage_settings: &StorageSettingsSnapshot,
) -> anyhow::Result<()> {
    let manifest = capture_backup_manifest(
        filename,
        database_kind,
        semantics,
        created_at,
        storage_settings,
        true,
        true,
    )
    .await;
    write_backup_manifest(&manifest).await
}

fn backup_command_timeout() -> Duration {
    std::env::var("VANSOUR_IMAGE_BACKUP_COMMAND_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map(Duration::from_secs)
        .unwrap_or(DEFAULT_BACKUP_COMMAND_TIMEOUT)
}

async fn run_streaming_dump_command(
    command_name: &str,
    mut command: Command,
    output_path: &Path,
) -> anyhow::Result<ExternalCommandOutcome> {
    let mut child = spawn_dump_command(command_name, &mut command)?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("{} 未提供可读取的 stdout", command_name))?;
    let stderr_task = capture_stderr_task(command_name, &mut child)?;
    let timeout = backup_command_timeout();
    let mut file = tokio::fs::File::create(output_path).await?;

    match tokio::time::timeout(timeout, tokio::io::copy(&mut stdout, &mut file)).await {
        Ok(Ok(_)) => {
            file.flush().await?;
        }
        Ok(Err(error)) => {
            kill_dump_process(command_name, &mut child).await;
            let _ = collect_stderr_excerpt(stderr_task).await;
            cleanup_backup_file(output_path).await;
            return Err(anyhow::anyhow!(
                "{} 输出流写入失败: {}",
                command_name,
                error
            ));
        }
        Err(_) => {
            kill_dump_process(command_name, &mut child).await;
            let _ = collect_stderr_excerpt(stderr_task).await;
            cleanup_backup_file(output_path).await;
            return Err(anyhow::anyhow!(
                "{} 超时，{} 秒内未完成",
                command_name,
                timeout.as_secs()
            ));
        }
    }

    let stderr_excerpt =
        finalize_dump_command(command_name, child, stderr_task, output_path).await?;
    Ok(ExternalCommandOutcome { stderr_excerpt })
}

async fn run_dump_command_with_result_file(
    command_name: &str,
    mut command: Command,
    output_path: &Path,
) -> anyhow::Result<ExternalCommandOutcome> {
    let mut child = spawn_dump_command(command_name, &mut command)?;
    let stderr_task = capture_stderr_task(command_name, &mut child)?;
    let stderr_excerpt =
        finalize_dump_command(command_name, child, stderr_task, output_path).await?;
    Ok(ExternalCommandOutcome { stderr_excerpt })
}

fn spawn_dump_command(command_name: &str, command: &mut Command) -> anyhow::Result<Child> {
    command
        .spawn()
        .map_err(|error| anyhow::anyhow!("无法启动 {}: {}", command_name, error))
}

fn capture_stderr_task(
    command_name: &str,
    child: &mut Child,
) -> anyhow::Result<tokio::task::JoinHandle<Vec<u8>>> {
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("{} 未提供可读取的 stderr", command_name))?;
    Ok(tokio::spawn(async move {
        let mut buffer = Vec::new();
        let _ = stderr.read_to_end(&mut buffer).await;
        buffer
    }))
}

async fn finalize_dump_command(
    command_name: &str,
    mut child: Child,
    stderr_task: tokio::task::JoinHandle<Vec<u8>>,
    output_path: &Path,
) -> anyhow::Result<Option<String>> {
    let timeout = backup_command_timeout();
    let status = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) => status,
        Ok(Err(error)) => {
            cleanup_backup_file(output_path).await;
            let _ = collect_stderr_excerpt(stderr_task).await;
            return Err(anyhow::anyhow!("等待 {} 结束失败: {}", command_name, error));
        }
        Err(_) => {
            kill_dump_process(command_name, &mut child).await;
            cleanup_backup_file(output_path).await;
            let _ = collect_stderr_excerpt(stderr_task).await;
            return Err(anyhow::anyhow!(
                "{} 超时，{} 秒内未退出",
                command_name,
                timeout.as_secs()
            ));
        }
    };

    let stderr_excerpt = collect_stderr_excerpt(stderr_task).await;
    if !status.success() {
        cleanup_backup_file(output_path).await;
        return Err(anyhow::anyhow!(
            "{} 执行失败: {}",
            command_name,
            stderr_excerpt.clone().unwrap_or_else(|| status.to_string())
        ));
    }
    if let Some(stderr_excerpt) = stderr_excerpt.as_ref() {
        warn!(
            "{} completed with warnings: {}",
            command_name, stderr_excerpt
        );
    }

    Ok(stderr_excerpt)
}

async fn collect_stderr_excerpt(stderr_task: tokio::task::JoinHandle<Vec<u8>>) -> Option<String> {
    match stderr_task.await {
        Ok(bytes) => process_output_excerpt(&bytes),
        Err(error) => Some(format!("stderr capture failed: {}", error)),
    }
}

async fn kill_dump_process(command_name: &str, child: &mut Child) {
    if let Err(error) = child.kill().await {
        warn!("failed to kill {} after error: {}", command_name, error);
    }
    let _ = child.wait().await;
}

async fn cleanup_backup_file(path: &Path) {
    let _ = tokio::fs::remove_file(path).await;
}

fn pg_dump_binary() -> anyhow::Result<String> {
    override_or_binary("VANSOUR_IMAGE_PG_DUMP_BIN", &["pg_dump"])
        .ok_or_else(|| anyhow::anyhow!("未找到 pg_dump"))
}

fn mysql_dump_binary() -> anyhow::Result<String> {
    override_or_binary(
        "VANSOUR_IMAGE_MYSQL_DUMP_BIN",
        &["mysqldump", "mariadb-dump"],
    )
    .ok_or_else(|| anyhow::anyhow!("未找到 mysqldump 或 mariadb-dump"))
}

fn override_or_binary(env_key: &str, candidates: &[&str]) -> Option<String> {
    if let Some(path) = std::env::var_os(env_key)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
    {
        return Some(path.to_string_lossy().into_owned());
    }
    find_first_binary(candidates)
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

fn parse_mysql_dump_target(database_url: &str) -> anyhow::Result<MySqlDumpTarget> {
    let normalized = normalize_mysql_compatible_url(database_url);
    let url = Url::parse(&normalized)
        .map_err(|error| anyhow::anyhow!("MySQL/MariaDB 连接地址解析失败: {}", error))?;
    if url.scheme() != "mysql" {
        anyhow::bail!("MySQL / MariaDB 备份只支持 mysql:// 或 mariadb:// 连接地址");
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

fn mysql_local_ssl_disable_args(_target: &MySqlDumpTarget) -> &'static [&'static str] {
    &[]
}

fn backup_path(filename: &str) -> Result<PathBuf, AppError> {
    if !is_valid_backup_filename(filename) {
        return Err(AppError::ValidationError("备份文件名无效".to_string()));
    }

    Ok(backup_directory().join(filename))
}

fn file_timestamp(metadata: &std::fs::Metadata) -> Option<DateTime<Utc>> {
    metadata
        .modified()
        .or_else(|_| metadata.created())
        .ok()
        .map(DateTime::<Utc>::from)
}

async fn ensure_nonempty_backup_file(path: &Path) -> anyhow::Result<u64> {
    let metadata = tokio::fs::metadata(path).await?;
    if metadata.len() == 0 {
        anyhow::bail!("备份文件为空，已拒绝保留");
    }
    Ok(metadata.len())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DatabaseKind};
    use crate::models::ComponentStatus;
    use crate::runtime_settings::{RuntimeSettings, StorageBackend};
    use crate::storage_backend::StorageManager;
    use std::ffi::OsString;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::{Arc, OnceLock};
    use tempfile::TempDir;
    use tokio::sync::Mutex;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct ScopedEnv {
        previous: Vec<(String, Option<OsString>)>,
    }

    impl ScopedEnv {
        fn set(key: &str, value: impl Into<OsString>) -> Self {
            let previous = std::env::var_os(key);
            #[allow(unused_unsafe)]
            unsafe {
                std::env::set_var(key, value.into());
            }
            Self {
                previous: vec![(key.to_string(), previous)],
            }
        }

        fn and_set(mut self, key: &str, value: impl Into<OsString>) -> Self {
            let previous = std::env::var_os(key);
            #[allow(unused_unsafe)]
            unsafe {
                std::env::set_var(key, value.into());
            }
            self.previous.push((key.to_string(), previous));
            self
        }
    }

    impl Drop for ScopedEnv {
        fn drop(&mut self) {
            for (key, previous) in self.previous.drain(..).rev() {
                #[allow(unused_unsafe)]
                unsafe {
                    match previous {
                        Some(previous) => std::env::set_var(&key, previous),
                        None => std::env::remove_var(&key),
                    }
                }
            }
        }
    }

    fn sample_runtime_settings(local_storage_path: String) -> RuntimeSettings {
        RuntimeSettings {
            site_name: "Avenrixa".to_string(),
            storage_backend: StorageBackend::Local,
            local_storage_path,
            mail_enabled: false,
            mail_smtp_host: String::new(),
            mail_smtp_port: 587,
            mail_smtp_user: None,
            mail_smtp_password: None,
            mail_from_email: String::new(),
            mail_from_name: String::new(),
            mail_link_base_url: String::new(),
            s3_endpoint: None,
            s3_region: None,
            s3_bucket: None,
            s3_prefix: None,
            s3_access_key: None,
            s3_secret_key: None,
            s3_force_path_style: true,
        }
    }

    fn build_postgres_backup_service(temp_dir: &TempDir) -> AdminDomainService {
        let mut config = Config::default();
        config.database.kind = DatabaseKind::Postgres;
        config.database.url = "postgres://localhost/test".to_string();
        config.storage.path = temp_dir
            .path()
            .join("storage")
            .to_string_lossy()
            .into_owned();

        let database = DatabasePool::Postgres(
            sqlx::PgPool::connect_lazy("postgres://localhost/test")
                .expect("lazy postgres pool should be created"),
        );
        let storage_manager = Arc::new(StorageManager::new(sample_runtime_settings(
            config.storage.path.clone(),
        )));

        AdminDomainService::new(
            database,
            None,
            ComponentStatus::healthy(),
            config,
            storage_manager,
        )
    }

    fn write_script(path: &Path, body: &str) {
        std::fs::write(path, body).expect("script should be written");
        let mut permissions = std::fs::metadata(path)
            .expect("script metadata should load")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("script should be executable");
    }

    #[tokio::test]
    async fn postgres_backup_streams_stdout_to_file_and_persists_manifest() {
        let _env_guard = env_lock().lock().await;
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let backup_dir = temp_dir.path().join("backups");
        let script_path = temp_dir.path().join("fake-pg-dump.sh");
        write_script(
            &script_path,
            "#!/bin/sh\nprintf 'CREATE TABLE demo(id int);\\n'\n",
        );
        let _env = ScopedEnv::set("VANSOUR_IMAGE_BACKUP_DIR", backup_dir.as_os_str())
            .and_set("VANSOUR_IMAGE_PG_DUMP_BIN", script_path.as_os_str());

        let service = build_postgres_backup_service(&temp_dir);
        let response = service
            .backup_database(Uuid::new_v4(), "admin@example.com")
            .await
            .expect("backup should succeed");

        let backup_file = backup_dir.join(&response.filename);
        let manifest_file = backup_dir.join(format!("{}.manifest.json", response.filename));
        let contents = tokio::fs::read_to_string(&backup_file)
            .await
            .expect("backup file should exist");

        assert_eq!(contents, "CREATE TABLE demo(id int);\n");
        assert!(
            tokio::fs::try_exists(&manifest_file)
                .await
                .expect("manifest should exist")
        );
    }

    #[tokio::test]
    async fn postgres_backup_timeout_removes_partial_file() {
        let _env_guard = env_lock().lock().await;
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let backup_dir = temp_dir.path().join("backups");
        let script_path = temp_dir.path().join("fake-pg-dump-timeout.sh");
        write_script(
            &script_path,
            "#!/bin/sh\nprintf 'partial dump'; sleep 2; printf 'late tail'\n",
        );
        let _env = ScopedEnv::set("VANSOUR_IMAGE_BACKUP_DIR", backup_dir.as_os_str())
            .and_set("VANSOUR_IMAGE_PG_DUMP_BIN", script_path.as_os_str())
            .and_set("VANSOUR_IMAGE_BACKUP_COMMAND_TIMEOUT_SECS", "1");

        let service = build_postgres_backup_service(&temp_dir);
        let error = service
            .backup_database(Uuid::new_v4(), "admin@example.com")
            .await
            .expect_err("backup timeout should fail");

        assert!(matches!(error, AppError::Internal(_)));
        let mut entries = tokio::fs::read_dir(&backup_dir)
            .await
            .expect("backup dir should be readable");
        assert!(
            entries
                .next_entry()
                .await
                .expect("backup dir iteration should succeed")
                .is_none(),
            "timed out backup should not leave partial files"
        );
    }
}
