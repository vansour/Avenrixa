use std::path::{Path, PathBuf};

use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::config::{Builder as S3ConfigBuilder, Region};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::DatabaseKind;
use crate::models::{
    BackupMetadataManifest, BackupObjectRollbackAnchor, BackupRestoreStorageSummary,
};
use crate::runtime_settings::{StorageBackend, StorageSettingsSnapshot};

const BACKUP_DIR: &str = "/data/backup";

pub async fn capture_backup_manifest(
    filename: &str,
    database_kind: DatabaseKind,
    created_at: DateTime<Utc>,
    storage_settings: &StorageSettingsSnapshot,
    app_installed: bool,
    has_admin: bool,
) -> BackupMetadataManifest {
    BackupMetadataManifest {
        filename: filename.to_string(),
        created_at,
        database_kind: database_kind.as_str().to_string(),
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

pub async fn load_backup_manifest(
    filename: &str,
) -> anyhow::Result<Option<BackupMetadataManifest>> {
    read_json_file(&backup_manifest_path(filename)).await
}

pub fn storage_signature(snapshot: &StorageSettingsSnapshot) -> String {
    let payload = serde_json::json!({
        "storage_backend": snapshot.storage_backend.as_str(),
        "local_storage_path": snapshot.local_storage_path,
        "s3_endpoint": snapshot.s3_endpoint,
        "s3_region": snapshot.s3_region,
        "s3_bucket": snapshot.s3_bucket,
        "s3_prefix": snapshot.s3_prefix,
        "s3_access_key": snapshot.s3_access_key,
        "s3_secret_key": snapshot.s3_secret_key,
        "s3_force_path_style": snapshot.s3_force_path_style,
    });

    blake3::hash(payload.to_string().as_bytes())
        .to_hex()
        .to_string()
}

fn backup_manifest_path(filename: &str) -> PathBuf {
    PathBuf::from(BACKUP_DIR).join(format!("{filename}.manifest.json"))
}

fn storage_summary(snapshot: &StorageSettingsSnapshot) -> BackupRestoreStorageSummary {
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

async fn capture_object_rollback_anchor(
    settings: &StorageSettingsSnapshot,
    checkpoint_at: DateTime<Utc>,
) -> BackupObjectRollbackAnchor {
    match settings.storage_backend {
        StorageBackend::Local => BackupObjectRollbackAnchor {
            strategy: "local-directory-snapshot".to_string(),
            checkpoint_at,
            local_storage_path: Some(settings.local_storage_path.clone()),
            s3_endpoint: None,
            s3_region: None,
            s3_bucket: None,
            s3_prefix: None,
            s3_force_path_style: true,
            s3_bucket_versioning_status: None,
            capture_error: None,
        },
        StorageBackend::S3 => {
            let (status, capture_error) = match fetch_s3_bucket_versioning_status(settings).await {
                Ok(status) => (Some(status), None),
                Err(error) => (None, Some(error.to_string())),
            };

            BackupObjectRollbackAnchor {
                strategy: "s3-versioned-rollback-anchor".to_string(),
                checkpoint_at,
                local_storage_path: None,
                s3_endpoint: settings.s3_endpoint.clone(),
                s3_region: settings.s3_region.clone(),
                s3_bucket: settings.s3_bucket.clone(),
                s3_prefix: settings.s3_prefix.clone(),
                s3_force_path_style: settings.s3_force_path_style,
                s3_bucket_versioning_status: status,
                capture_error,
            }
        }
    }
}

async fn fetch_s3_bucket_versioning_status(
    settings: &StorageSettingsSnapshot,
) -> anyhow::Result<String> {
    let endpoint = required_s3_setting(settings.s3_endpoint.as_deref(), "endpoint")?;
    let region = required_s3_setting(settings.s3_region.as_deref(), "region")?;
    let bucket = required_s3_setting(settings.s3_bucket.as_deref(), "bucket")?;
    let access_key = required_s3_setting(settings.s3_access_key.as_deref(), "access key")?;
    let secret_key = required_s3_setting(settings.s3_secret_key.as_deref(), "secret key")?;

    let creds = Credentials::new(access_key, secret_key, None, None, "backup-manifest");
    let mut builder = S3ConfigBuilder::new()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new(region))
        .credentials_provider(creds)
        .endpoint_url(endpoint);
    if settings.s3_force_path_style {
        builder = builder.force_path_style(true);
    }
    let client = S3Client::from_conf(builder.build());
    let response = client.get_bucket_versioning().bucket(bucket).send().await?;

    Ok(response
        .status()
        .map(|status| status.as_str().to_ascii_lowercase())
        .unwrap_or_else(|| "disabled".to_string()))
}

fn required_s3_setting(value: Option<&str>, name: &str) -> anyhow::Result<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow::anyhow!("S3 配置缺少必填项: {}", name))
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
