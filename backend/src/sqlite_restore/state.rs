use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::backup_manifest::backup_directory;
use crate::config::DatabaseKind;
use crate::models::{
    BackupDatabaseFamily, BackupFileSummary, BackupObjectRollbackAnchor,
    BackupObjectRollbackStrategy, BackupRestoreResult, BackupRestoreStorageSummary,
    PendingBackupRestore, StorageBackendKind, config_database_kind_from_backup_family,
    infer_backup_semantics,
};
use crate::runtime_settings::StorageSettingsSnapshot;

const DEFAULT_PENDING_RESTORE_PATH: &str = "/data/backup/pending_restore.json";
const DEFAULT_LAST_RESTORE_RESULT_PATH: &str = "/data/backup/last_restore_result.json";

pub(super) fn storage_summary_from_snapshot(
    snapshot: &StorageSettingsSnapshot,
) -> BackupRestoreStorageSummary {
    BackupRestoreStorageSummary {
        storage_backend: crate::models::storage_backend_kind_from_runtime(snapshot.storage_backend),
        local_storage_path: snapshot.local_storage_path.clone(),
        s3_endpoint: snapshot.s3_endpoint.clone(),
        s3_region: snapshot.s3_region.clone(),
        s3_bucket: snapshot.s3_bucket.clone(),
        s3_prefix: snapshot.s3_prefix.clone(),
        s3_force_path_style: snapshot.s3_force_path_style,
    }
}

pub(super) fn unknown_storage_summary() -> BackupRestoreStorageSummary {
    BackupRestoreStorageSummary {
        storage_backend: StorageBackendKind::Unknown,
        local_storage_path: String::new(),
        s3_endpoint: None,
        s3_region: None,
        s3_bucket: None,
        s3_prefix: None,
        s3_force_path_style: true,
    }
}

pub(super) fn backup_database_kind_from_pending(
    pending: &PendingBackupRestore,
) -> Option<DatabaseKind> {
    config_database_kind_from_backup_family(pending.database_kind)
        .or_else(|| config_database_kind_from_backup_family(pending.semantics.database_family))
        .or_else(|| backup_database_kind_from_filename(&pending.filename))
}

pub(super) fn backup_database_kind_from_filename(filename: &str) -> Option<DatabaseKind> {
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

pub(super) fn restore_database_label(database_kind: BackupDatabaseFamily) -> &'static str {
    database_kind.label()
}

pub(super) fn append_object_rollback_anchor_warnings(
    anchor: Option<&BackupObjectRollbackAnchor>,
    warnings: &mut Vec<String>,
) {
    let Some(anchor) = anchor else {
        return;
    };

    match anchor.strategy {
        BackupObjectRollbackStrategy::LocalDirectorySnapshot => {
            if let Some(path) = anchor.local_storage_path.as_deref() {
                warnings.push(format!(
                    "这份备份绑定的文件回滚锚点目录为 {}。如需回退本地附件，请按相同时间点恢复该目录快照。",
                    path
                ));
            }
        }
        BackupObjectRollbackStrategy::S3VersionedRollbackAnchor => {
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
        BackupObjectRollbackStrategy::Unknown => {}
    }
}

pub(super) async fn load_pending_restore_plan() -> anyhow::Result<Option<PendingBackupRestore>> {
    Ok(read_json_file(&pending_restore_path())
        .await?
        .map(normalize_pending_restore))
}

pub(super) async fn persist_pending_restore_plan(
    pending: &PendingBackupRestore,
) -> anyhow::Result<()> {
    write_json_file(&pending_restore_path(), pending).await
}

pub(super) async fn load_last_restore_result() -> anyhow::Result<Option<BackupRestoreResult>> {
    Ok(read_json_file(&last_restore_result_path())
        .await?
        .map(normalize_restore_result))
}

pub(super) async fn persist_last_restore_result(
    result: &BackupRestoreResult,
) -> anyhow::Result<()> {
    write_json_file(&last_restore_result_path(), result).await
}

pub(super) async fn clear_pending_restore_plan() -> anyhow::Result<()> {
    let path = pending_restore_path();
    if tokio::fs::try_exists(&path).await.unwrap_or(false) {
        tokio::fs::remove_file(path).await?;
    }
    Ok(())
}

pub(super) fn backup_path(filename: &str) -> Result<PathBuf, crate::error::AppError> {
    if !validate_backup_filename(filename) {
        return Err(crate::error::AppError::ValidationError(
            "备份文件名无效".to_string(),
        ));
    }

    Ok(backup_directory().join(filename))
}

pub(super) async fn backup_file_summary(
    filename: &str,
) -> Result<BackupFileSummary, crate::error::AppError> {
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
        semantics: infer_backup_semantics(filename, backup_database_kind_from_filename(filename)),
    })
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

fn normalize_pending_restore(mut pending: PendingBackupRestore) -> PendingBackupRestore {
    if pending.semantics.is_unknown() {
        pending.semantics = infer_backup_semantics(
            &pending.filename,
            config_database_kind_from_backup_family(pending.database_kind),
        );
    }
    pending
}

fn normalize_restore_result(mut result: BackupRestoreResult) -> BackupRestoreResult {
    if result.semantics.is_unknown() {
        result.semantics = infer_backup_semantics(
            &result.filename,
            config_database_kind_from_backup_family(result.database_kind),
        );
    }
    result
}
