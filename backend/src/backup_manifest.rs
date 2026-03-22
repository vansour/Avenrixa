use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::config::DatabaseKind;
use crate::models::{
    BackupMetadataManifest, BackupObjectRollbackAnchor, BackupObjectRollbackStrategy,
    BackupRestoreStorageSummary, BackupSemantics, backup_database_family_from_config,
};
use crate::runtime_settings::{StorageBackend, StorageSettingsSnapshot};

const DEFAULT_BACKUP_DIR: &str = "/data/backup";
const CURRENT_BACKUP_MANIFEST_FORMAT_VERSION: u32 = 2;

pub async fn capture_backup_manifest(
    filename: &str,
    database_kind: DatabaseKind,
    semantics: BackupSemantics,
    created_at: DateTime<Utc>,
    storage_settings: &StorageSettingsSnapshot,
    app_installed: bool,
    has_admin: bool,
) -> BackupMetadataManifest {
    BackupMetadataManifest {
        format_version: CURRENT_BACKUP_MANIFEST_FORMAT_VERSION,
        filename: filename.to_string(),
        created_at,
        database_kind: backup_database_family_from_config(database_kind),
        semantics,
        app_installed,
        has_admin,
        storage_signature: storage_signature(storage_settings),
        storage: storage_summary(storage_settings),
        object_rollback_anchor: capture_object_rollback_anchor(storage_settings, created_at).await,
    }
}

pub async fn write_backup_manifest(manifest: &BackupMetadataManifest) -> anyhow::Result<()> {
    write_json_file(&backup_manifest_path(&manifest.filename), manifest).await
}

pub fn storage_signature(snapshot: &StorageSettingsSnapshot) -> String {
    let payload = serde_json::json!({
        "storage_backend": snapshot.storage_backend.as_str(),
        "local_storage_path": snapshot.local_storage_path,
    });

    blake3::hash(payload.to_string().as_bytes())
        .to_hex()
        .to_string()
}

pub(crate) fn backup_directory() -> PathBuf {
    std::env::var("AVENRIXA_BACKUP_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_BACKUP_DIR))
}

fn backup_manifest_path(filename: &str) -> PathBuf {
    backup_directory().join(format!("{filename}.manifest.json"))
}

fn storage_summary(snapshot: &StorageSettingsSnapshot) -> BackupRestoreStorageSummary {
    BackupRestoreStorageSummary {
        storage_backend: crate::models::storage_backend_kind_from_runtime(snapshot.storage_backend),
        local_storage_path: snapshot.local_storage_path.clone(),
    }
}

async fn capture_object_rollback_anchor(
    settings: &StorageSettingsSnapshot,
    checkpoint_at: DateTime<Utc>,
) -> BackupObjectRollbackAnchor {
    match settings.storage_backend {
        StorageBackend::Local => BackupObjectRollbackAnchor {
            strategy: BackupObjectRollbackStrategy::LocalDirectorySnapshot,
            checkpoint_at,
            local_storage_path: Some(settings.local_storage_path.clone()),
            capture_error: None,
        },
    }
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

#[cfg(test)]
fn normalize_backup_manifest(
    filename: &str,
    mut manifest: BackupMetadataManifest,
) -> BackupMetadataManifest {
    if manifest.semantics.is_unknown() {
        manifest.semantics = crate::models::infer_backup_semantics(
            filename,
            crate::models::config_database_kind_from_backup_family(manifest.database_kind),
        );
    }
    if manifest.format_version == 0 {
        manifest.format_version = 1;
    }
    manifest
}

#[cfg(test)]
mod tests {
    use super::normalize_backup_manifest;
    use crate::models::{
        BackupDatabaseFamily, BackupMetadataManifest, BackupObjectRollbackAnchor,
        BackupObjectRollbackStrategy, BackupRestoreStorageSummary, BackupSemantics,
        StorageBackendKind,
    };
    use chrono::Utc;
    use shared_types::backup::BackupKind;

    #[test]
    fn legacy_manifest_is_upgraded_with_inferred_semantics() {
        let manifest = BackupMetadataManifest {
            format_version: 1,
            filename: "backup_legacy.postgresql.sql".to_string(),
            created_at: Utc::now(),
            database_kind: BackupDatabaseFamily::Postgres,
            semantics: BackupSemantics::default(),
            app_installed: true,
            has_admin: true,
            storage_signature: "sig".to_string(),
            storage: BackupRestoreStorageSummary {
                storage_backend: StorageBackendKind::Local,
                local_storage_path: "/data/images".to_string(),
            },
            object_rollback_anchor: BackupObjectRollbackAnchor {
                strategy: BackupObjectRollbackStrategy::LocalDirectorySnapshot,
                checkpoint_at: Utc::now(),
                local_storage_path: Some("/data/images".to_string()),
                capture_error: None,
            },
        };

        let filename = manifest.filename.clone();
        let normalized = normalize_backup_manifest(&filename, manifest);
        assert_eq!(
            normalized.semantics.database_family,
            BackupDatabaseFamily::Postgres
        );
        assert_eq!(
            normalized.semantics.backup_kind,
            BackupKind::PostgresqlLogicalDump
        );
        assert!(!normalized.semantics.ui_restore_supported);
    }
}
