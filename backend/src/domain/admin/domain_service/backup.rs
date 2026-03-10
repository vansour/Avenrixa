use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{error, info};
use uuid::Uuid;

use super::AdminDomainService;
use crate::audit::log_audit_db;
use crate::db::DatabasePool;
use crate::error::AppError;
use crate::models::{BackupFileSummary, BackupResponse};

const BACKUP_DIR: &str = "/data/backup";

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
        if !tokio::fs::try_exists(BACKUP_DIR).await.unwrap_or(false) {
            return Ok(Vec::new());
        }

        let mut directory = tokio::fs::read_dir(BACKUP_DIR).await?;
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

            backups.push(BackupFileSummary {
                filename,
                created_at,
                size_bytes: metadata.len(),
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
        tokio::fs::create_dir_all(BACKUP_DIR).await?;

        match &self.database {
            DatabasePool::Postgres(_) => {
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
                    }),
                );

                Ok(BackupResponse {
                    filename,
                    created_at: Utc::now(),
                })
            }
            DatabasePool::Sqlite(pool) => {
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

                let backup_size_bytes = tokio::fs::metadata(&backup_path)
                    .await
                    .map(|meta| meta.len())
                    .unwrap_or(0);

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
                        "backup_size_bytes": backup_size_bytes,
                    }),
                );

                Ok(BackupResponse {
                    filename,
                    created_at: Utc::now(),
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
        && (filename.ends_with(".sql") || filename.ends_with(".sqlite3"))
        && filename.bytes().all(|byte| {
            matches!(
                byte,
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'_' | b'-'
            )
        })
}

fn backup_path(filename: &str) -> Result<PathBuf, AppError> {
    if !is_valid_backup_filename(filename) {
        return Err(AppError::ValidationError("备份文件名无效".to_string()));
    }

    Ok(Path::new(BACKUP_DIR).join(filename))
}

fn file_timestamp(metadata: &std::fs::Metadata) -> Option<DateTime<Utc>> {
    metadata
        .modified()
        .or_else(|_| metadata.created())
        .ok()
        .map(DateTime::<Utc>::from)
}
