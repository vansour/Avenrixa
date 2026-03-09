use aws_sdk_s3::primitives::ByteStream;

use crate::error::AppError;
use crate::runtime_settings::RuntimeSettings;
use crate::storage_backend::StorageManager;
use crate::storage_backend::path::build_s3_object_key;

pub(super) async fn exists(
    manager: &StorageManager,
    settings: &RuntimeSettings,
    file_key: &str,
) -> Result<bool, AppError> {
    let cached = manager.resolve_s3_client(settings).await?;
    let object_key = build_s3_object_key(cached.prefix.as_deref(), file_key);
    match cached
        .client
        .head_object()
        .bucket(cached.bucket.clone())
        .key(object_key)
        .send()
        .await
    {
        Ok(_) => Ok(true),
        Err(error) => {
            if let Some(service_error) = error.as_service_error()
                && service_error.is_not_found()
            {
                return Ok(false);
            }
            Err(s3_error("check S3 object existence", error))
        }
    }
}

pub(super) async fn read(
    manager: &StorageManager,
    settings: &RuntimeSettings,
    file_key: &str,
) -> Result<Vec<u8>, AppError> {
    let cached = manager.resolve_s3_client(settings).await?;
    let object_key = build_s3_object_key(cached.prefix.as_deref(), file_key);
    let response = cached
        .client
        .get_object()
        .bucket(cached.bucket.clone())
        .key(object_key)
        .send()
        .await
        .map_err(|error| s3_error("read S3 object", error))?;
    let bytes = response
        .body
        .collect()
        .await
        .map_err(|error| s3_error("collect S3 bytes", error))?
        .into_bytes();
    Ok(bytes.to_vec())
}

pub(super) async fn write(
    manager: &StorageManager,
    settings: &RuntimeSettings,
    file_key: &str,
    data: &[u8],
) -> Result<(), AppError> {
    let cached = manager.resolve_s3_client(settings).await?;
    let object_key = build_s3_object_key(cached.prefix.as_deref(), file_key);
    cached
        .client
        .put_object()
        .bucket(cached.bucket.clone())
        .key(object_key)
        .body(ByteStream::from(data.to_vec()))
        .send()
        .await
        .map_err(|error| s3_error("write S3 object", error))?;
    Ok(())
}

pub(super) async fn delete(
    manager: &StorageManager,
    settings: &RuntimeSettings,
    file_key: &str,
) -> Result<(), AppError> {
    let cached = manager.resolve_s3_client(settings).await?;
    let object_key = build_s3_object_key(cached.prefix.as_deref(), file_key);
    cached
        .client
        .delete_object()
        .bucket(cached.bucket.clone())
        .key(object_key)
        .send()
        .await
        .map_err(|error| s3_error("delete S3 object", error))?;
    Ok(())
}

pub(super) async fn copy(
    manager: &StorageManager,
    settings: &RuntimeSettings,
    src_key: &str,
    dst_key: &str,
) -> Result<(), AppError> {
    let cached = manager.resolve_s3_client(settings).await?;
    let src_object_key = build_s3_object_key(cached.prefix.as_deref(), src_key);
    let dst_object_key = build_s3_object_key(cached.prefix.as_deref(), dst_key);
    let copy_source = format!("{}/{}", cached.bucket, src_object_key);
    cached
        .client
        .copy_object()
        .bucket(cached.bucket.clone())
        .key(dst_object_key)
        .copy_source(copy_source)
        .send()
        .await
        .map_err(|error| s3_error("copy S3 object", error))?;
    Ok(())
}

fn s3_error(action: &str, error: impl std::fmt::Display) -> AppError {
    AppError::Internal(anyhow::anyhow!("Failed to {}: {}", action, error))
}
