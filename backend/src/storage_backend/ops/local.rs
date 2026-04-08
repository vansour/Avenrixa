use crate::error::AppError;
use crate::runtime_settings::RuntimeSettings;
use crate::storage_backend::StorageReadHandle;
use crate::storage_backend::path::{local_path_candidates, primary_local_path};
use tokio::fs;

pub(super) async fn exists(settings: &RuntimeSettings, file_key: &str) -> Result<bool, AppError> {
    for path in local_path_candidates(&settings.local_storage_path, file_key)? {
        if fs::try_exists(path).await.unwrap_or(false) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) async fn read(settings: &RuntimeSettings, file_key: &str) -> Result<Vec<u8>, AppError> {
    for path in local_path_candidates(&settings.local_storage_path, file_key)? {
        match fs::read(&path).await {
            Ok(bytes) => return Ok(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(AppError::IoError(error)),
        }
    }

    Err(AppError::IoError(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("file key not found: {file_key}"),
    )))
}

pub(super) async fn open_read(
    settings: &RuntimeSettings,
    file_key: &str,
) -> Result<StorageReadHandle, AppError> {
    for path in local_path_candidates(&settings.local_storage_path, file_key)? {
        match fs::File::open(&path).await {
            Ok(file) => {
                let metadata = file.metadata().await?;
                return Ok(StorageReadHandle {
                    file,
                    content_length: metadata.len(),
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(AppError::IoError(error)),
        }
    }

    Err(AppError::IoError(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("file key not found: {file_key}"),
    )))
}

pub(super) async fn write(
    settings: &RuntimeSettings,
    file_key: &str,
    data: &[u8],
) -> Result<(), AppError> {
    let path = primary_local_path(&settings.local_storage_path, file_key)?;
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
    let mut removed_any = false;
    let mut last_error = None;

    for path in local_path_candidates(local_storage_path, file_key)? {
        match fs::remove_file(path).await {
            Ok(_) => {
                removed_any = true;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => last_error = Some(error),
        }
    }

    if let Some(error) = last_error {
        return Err(AppError::IoError(error));
    }

    let _ = removed_any;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_settings::StorageBackend;

    fn sample_runtime_settings(local_storage_path: String) -> RuntimeSettings {
        RuntimeSettings {
            site_name: "Avenrixa".to_string(),
            storage_backend: StorageBackend::Local,
            local_storage_path,
            mail_enabled: false,
            mail_smtp_host: String::new(),
            mail_smtp_port: 587,
            mail_smtp_user: None,
            mail_smtp_password: None,
            mail_from_email: String::new(),
            mail_from_name: String::new(),
            mail_link_base_url: String::new(),
        }
    }

    #[tokio::test]
    async fn write_places_hash_named_files_into_sharded_layout() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let settings = sample_runtime_settings(temp_dir.path().to_string_lossy().into_owned());
        let file_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.png";

        write(&settings, file_key, b"payload")
            .await
            .expect("write should succeed");

        let sharded_path = primary_local_path(&settings.local_storage_path, file_key)
            .expect("primary path should build");
        assert!(
            fs::try_exists(&sharded_path)
                .await
                .expect("path existence should succeed")
        );
    }

    #[tokio::test]
    async fn open_read_supports_legacy_flat_layout() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let settings = sample_runtime_settings(temp_dir.path().to_string_lossy().into_owned());
        let file_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.png";
        let legacy_path = temp_dir.path().join(file_key);
        fs::write(&legacy_path, b"legacy")
            .await
            .expect("legacy file should be written");

        let handle = open_read(&settings, file_key)
            .await
            .expect("legacy file should be readable");

        assert_eq!(handle.content_length, 6);
    }

    #[tokio::test]
    async fn delete_removes_both_legacy_and_sharded_paths() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let settings = sample_runtime_settings(temp_dir.path().to_string_lossy().into_owned());
        let file_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.png";
        let legacy_path = temp_dir.path().join(file_key);
        let sharded_path = primary_local_path(&settings.local_storage_path, file_key)
            .expect("primary path should build");

        if let Some(parent) = sharded_path.parent() {
            fs::create_dir_all(parent)
                .await
                .expect("sharded parent should exist");
        }
        fs::write(&legacy_path, b"legacy")
            .await
            .expect("legacy file should be written");
        fs::write(&sharded_path, b"sharded")
            .await
            .expect("sharded file should be written");

        delete_with_base_path(&settings.local_storage_path, file_key)
            .await
            .expect("delete should succeed");

        assert!(
            !fs::try_exists(&legacy_path)
                .await
                .expect("legacy existence should succeed")
        );
        assert!(
            !fs::try_exists(&sharded_path)
                .await
                .expect("sharded existence should succeed")
        );
    }
}
