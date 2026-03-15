use chrono::{DateTime, Utc};
use reqwest::Url;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{error, info, warn};
use uuid::Uuid;

use super::AdminDomainService;
use crate::audit::log_audit_db;
use crate::backup_manifest::{backup_directory, capture_backup_manifest, write_backup_manifest};
use crate::config::{DatabaseKind, normalize_mysql_compatible_url};
use crate::db::DatabasePool;
use crate::error::AppError;
use crate::models::{BackupFileSummary, BackupResponse, BackupSemantics};
use crate::runtime_settings::StorageSettingsSnapshot;

struct MySqlDumpTarget {
    host: String,
    port: u16,
    username: String,
    password: Option<String>,
    database: String,
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
            let semantics = BackupSemantics::infer(&filename, None);

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
                let semantics = BackupSemantics::from_database_kind(DatabaseKind::Postgres);
                let filename = format!("backup_{}.sql", Uuid::new_v4());
                let backup_path = backup_path(&filename)?;
                let database = self.database.clone();
                let database_url = &self.config.database.url;
                let child = tokio::process::Command::new("pg_dump")
                    .arg("--dbname")
                    .arg(database_url)
                    .arg("--format=plain")
                    .arg("--no-owner")
                    .arg("--no-acl")
                    .stdout(std::process::Stdio::piped())
                    .spawn();

                let mut child = match child {
                    Ok(child) => child,
                    Err(e) => {
                        error!("Failed to spawn pg_dump process: {}", e);
                        spawn_backup_audit(
                            database,
                            admin_user_id,
                            "admin.maintenance.database_backup.failed",
                            serde_json::json!({
                                "admin_email": admin_email,
                                "error": e.to_string(),
                                "result": "failed",
                                "risk_level": "info",
                                "database_kind": "postgresql",
                            }),
                        );
                        return Err(AppError::Internal(anyhow::anyhow!(
                            "Failed to execute pg_dump: {}",
                            e
                        )));
                    }
                };

                let mut stdout = child.stdout.take().expect("Failed to capture stdout");
                let mut buffer = Vec::new();
                let _ = stdout.read_to_end(&mut buffer).await;

                let mut file = tokio::fs::File::create(&backup_path).await?;
                file.write_all(&buffer).await?;

                let status = child.wait().await;
                match status {
                    Ok(status) => {
                        if !status.success() {
                            error!("pg_dump failed with status: {}", status);
                            let _ = tokio::fs::remove_file(&backup_path).await;
                            spawn_backup_audit(
                                self.database.clone(),
                                admin_user_id,
                                "admin.maintenance.database_backup.failed",
                                serde_json::json!({
                                    "admin_email": admin_email,
                                    "error": status.to_string(),
                                    "result": "failed",
                                    "risk_level": "info",
                                    "database_kind": "postgresql",
                                }),
                            );
                            return Err(AppError::Internal(anyhow::anyhow!(
                                "pg_dump failed with exit code: {}",
                                status
                            )));
                        }
                    }
                    Err(e) => {
                        error!("Failed to wait for pg_dump: {}", e);
                        let _ = tokio::fs::remove_file(&backup_path).await;
                        spawn_backup_audit(
                            self.database.clone(),
                            admin_user_id,
                            "admin.maintenance.database_backup.failed",
                            serde_json::json!({
                                "admin_email": admin_email,
                                "error": e.to_string(),
                                "result": "failed",
                                "risk_level": "info",
                                "database_kind": "postgresql",
                            }),
                        );
                        return Err(AppError::Internal(anyhow::anyhow!(
                            "pg_dump wait error: {}",
                            e
                        )));
                    }
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
                    }),
                );

                Ok(BackupResponse {
                    filename,
                    created_at,
                    semantics,
                })
            }
            DatabasePool::MySql(_) => {
                let semantics = BackupSemantics::from_database_kind(DatabaseKind::MySql);
                let filename = format!("backup_{}.mysql.sql", Uuid::new_v4());
                let backup_path = backup_path(&filename)?;
                let database = self.database.clone();
                let dump_target = parse_mysql_dump_target(&self.config.database.url)
                    .map_err(AppError::Internal)?;
                let dump_bin = mysql_dump_binary().map_err(AppError::Internal)?;

                if tokio::fs::try_exists(&backup_path).await.unwrap_or(false) {
                    let _ = tokio::fs::remove_file(&backup_path).await;
                }

                let mut command = tokio::process::Command::new(&dump_bin);
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
                    .stderr(std::process::Stdio::piped());
                if let Some(password) = dump_target.password.as_ref() {
                    command.env("MYSQL_PWD", password);
                }

                let child = command.spawn();
                let child = match child {
                    Ok(child) => child,
                    Err(error) => {
                        error!("Failed to spawn mysqldump process: {}", error);
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
                        return Err(AppError::Internal(anyhow::anyhow!(
                            "Failed to execute mysqldump: {}",
                            error
                        )));
                    }
                };

                let output = child.wait_with_output().await;
                let warning_excerpt = match output {
                    Ok(output) => {
                        let stderr_excerpt = process_output_excerpt(&output.stderr);
                        if !output.status.success() {
                            error!("mysqldump failed with status: {}", output.status);
                            let _ = tokio::fs::remove_file(&backup_path).await;
                            spawn_backup_audit(
                                self.database.clone(),
                                admin_user_id,
                                "admin.maintenance.database_backup.failed",
                                serde_json::json!({
                                    "admin_email": admin_email,
                                    "error": stderr_excerpt
                                        .clone()
                                        .unwrap_or_else(|| output.status.to_string()),
                                    "result": "failed",
                                    "risk_level": "info",
                                    "database_kind": "mysql",
                                }),
                            );
                            return Err(AppError::Internal(anyhow::anyhow!(
                                "mysqldump failed: {}",
                                stderr_excerpt
                                    .clone()
                                    .unwrap_or_else(|| output.status.to_string())
                            )));
                        }
                        if let Some(stderr_excerpt) = stderr_excerpt.as_ref() {
                            warn!("mysqldump completed with warnings: {}", stderr_excerpt);
                        }
                        stderr_excerpt
                    }
                    Err(error) => {
                        error!("Failed to wait for mysqldump: {}", error);
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
                                "database_kind": "mysql",
                            }),
                        );
                        return Err(AppError::Internal(anyhow::anyhow!(
                            "mysqldump wait error: {}",
                            error
                        )));
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
                let semantics = BackupSemantics::from_database_kind(DatabaseKind::Sqlite);
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

fn mysql_dump_binary() -> anyhow::Result<String> {
    find_first_binary(&["mysqldump", "mariadb-dump"])
        .ok_or_else(|| anyhow::anyhow!("未找到 mysqldump 或 mariadb-dump"))
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
