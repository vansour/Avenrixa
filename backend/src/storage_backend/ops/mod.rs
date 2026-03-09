mod local;
mod s3_ops;

use super::StorageManager;
use super::path::validate_file_key;
use crate::error::AppError;
use crate::runtime_settings::StorageBackend;

impl StorageManager {
    pub async fn exists(&self, file_key: &str) -> Result<bool, AppError> {
        validate_file_key(file_key)?;
        let settings = self.runtime_settings().await?;
        match settings.storage_backend {
            StorageBackend::Local => local::exists(&settings, file_key).await,
            StorageBackend::S3 => s3_ops::exists(self, &settings, file_key).await,
        }
    }

    pub async fn read(&self, file_key: &str) -> Result<Vec<u8>, AppError> {
        validate_file_key(file_key)?;
        let settings = self.runtime_settings().await?;
        match settings.storage_backend {
            StorageBackend::Local => local::read(&settings, file_key).await,
            StorageBackend::S3 => s3_ops::read(self, &settings, file_key).await,
        }
    }

    pub async fn write(&self, file_key: &str, data: &[u8]) -> Result<(), AppError> {
        validate_file_key(file_key)?;
        let settings = self.runtime_settings().await?;
        match settings.storage_backend {
            StorageBackend::Local => local::write(&settings, file_key, data).await,
            StorageBackend::S3 => s3_ops::write(self, &settings, file_key, data).await,
        }
    }

    pub async fn delete(&self, file_key: &str) -> Result<(), AppError> {
        validate_file_key(file_key)?;
        let settings = self.runtime_settings().await?;
        match settings.storage_backend {
            StorageBackend::Local => local::delete(&settings, file_key).await,
            StorageBackend::S3 => s3_ops::delete(self, &settings, file_key).await,
        }
    }

    pub async fn copy(&self, src_key: &str, dst_key: &str) -> Result<(), AppError> {
        validate_file_key(src_key)?;
        validate_file_key(dst_key)?;
        let settings = self.runtime_settings().await?;
        match settings.storage_backend {
            StorageBackend::Local => local::copy(&settings, src_key, dst_key).await,
            StorageBackend::S3 => s3_ops::copy(self, &settings, src_key, dst_key).await,
        }
    }
}
