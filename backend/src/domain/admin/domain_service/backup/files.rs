use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};

use crate::backup_manifest::{backup_directory, capture_backup_manifest, write_backup_manifest};
use crate::config::DatabaseKind;
use crate::error::AppError;
use crate::models::BackupSemantics;
use crate::runtime_settings::StorageSettingsSnapshot;

pub(super) fn is_valid_backup_filename(filename: &str) -> bool {
    !filename.is_empty()
        && filename.len() <= 255
        && filename.starts_with("backup_")
        && filename.ends_with(".sql")
        && filename.bytes().all(|byte| {
            matches!(
                byte,
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'_' | b'-'
            )
        })
}

pub(super) async fn persist_backup_manifest(
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

pub(super) fn backup_path(filename: &str) -> Result<PathBuf, AppError> {
    if !is_valid_backup_filename(filename) {
        return Err(AppError::ValidationError("备份文件名无效".to_string()));
    }

    Ok(backup_directory().join(filename))
}

pub(super) fn file_timestamp(metadata: &std::fs::Metadata) -> Option<DateTime<Utc>> {
    metadata
        .modified()
        .or_else(|_| metadata.created())
        .ok()
        .map(DateTime::<Utc>::from)
}

pub(super) async fn ensure_nonempty_backup_file(path: &Path) -> anyhow::Result<u64> {
    let metadata = tokio::fs::metadata(path).await?;
    if metadata.len() == 0 {
        anyhow::bail!("备份文件为空，已拒绝保留");
    }
    Ok(metadata.len())
}
