use crate::error::AppError;
use crate::runtime_settings::RuntimeSettings;
use crate::storage_backend::path::join_local_path;
use tokio::fs;

pub(super) async fn exists(settings: &RuntimeSettings, file_key: &str) -> Result<bool, AppError> {
    let path = join_local_path(&settings.local_storage_path, file_key)?;
    Ok(fs::try_exists(path).await.unwrap_or(false))
}

pub(super) async fn read(settings: &RuntimeSettings, file_key: &str) -> Result<Vec<u8>, AppError> {
    let path = join_local_path(&settings.local_storage_path, file_key)?;
    Ok(fs::read(path).await?)
}

pub(super) async fn write(
    settings: &RuntimeSettings,
    file_key: &str,
    data: &[u8],
) -> Result<(), AppError> {
    let path = join_local_path(&settings.local_storage_path, file_key)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(path, data).await?;
    Ok(())
}

pub(super) async fn delete(settings: &RuntimeSettings, file_key: &str) -> Result<(), AppError> {
    let path = join_local_path(&settings.local_storage_path, file_key)?;
    match fs::remove_file(path).await {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::IoError(error)),
    }
}

pub(super) async fn copy(
    settings: &RuntimeSettings,
    src_key: &str,
    dst_key: &str,
) -> Result<(), AppError> {
    let src = join_local_path(&settings.local_storage_path, src_key)?;
    let dst = join_local_path(&settings.local_storage_path, dst_key)?;
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::copy(src, dst).await?;
    Ok(())
}
