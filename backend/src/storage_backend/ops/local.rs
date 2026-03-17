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
    delete_with_base_path(&settings.local_storage_path, file_key).await
}

pub(super) async fn delete_with_base_path(
    local_storage_path: &str,
    file_key: &str,
) -> Result<(), AppError> {
    let path = join_local_path(local_storage_path, file_key)?;
    match fs::remove_file(path).await {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::IoError(error)),
    }
}
