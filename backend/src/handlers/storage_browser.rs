use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::AppError;
use crate::models::{StorageDirectoryBrowseResponse, StorageDirectoryEntry};

#[derive(Debug, Deserialize)]
pub struct BrowseStorageDirectoriesQuery {
    pub path: Option<String>,
}

pub async fn browse_storage_directories(
    raw: Option<&str>,
) -> Result<StorageDirectoryBrowseResponse, AppError> {
    let current_path = resolve_storage_directory(raw).await?;
    let directories = list_storage_directories(&current_path).await?;

    Ok(StorageDirectoryBrowseResponse {
        current_path: path_to_string(&current_path),
        parent_path: current_path.parent().map(path_to_string),
        directories,
    })
}

async fn resolve_storage_directory(raw: Option<&str>) -> Result<PathBuf, AppError> {
    let requested = raw
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("/data");
    let mut candidate = PathBuf::from(requested);
    if !candidate.is_absolute() {
        return Err(AppError::ValidationError(
            "目录路径必须是绝对路径".to_string(),
        ));
    }

    loop {
        match tokio::fs::metadata(&candidate).await {
            Ok(metadata) if metadata.is_dir() => {
                return tokio::fs::canonicalize(&candidate)
                    .await
                    .map_err(AppError::from);
            }
            Ok(_) => {
                if let Some(parent) = candidate.parent() {
                    candidate = parent.to_path_buf();
                    continue;
                }
                return Err(AppError::ValidationError("目录不存在".to_string()));
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {
                if let Some(parent) = candidate.parent() {
                    candidate = parent.to_path_buf();
                    continue;
                }
                return Err(AppError::ValidationError("目录不存在".to_string()));
            }
            Err(error) if error.kind() == ErrorKind::PermissionDenied => {
                return Err(AppError::ValidationError(format!(
                    "目录不可访问: {}",
                    candidate.display()
                )));
            }
            Err(error) => return Err(AppError::IoError(error)),
        }
    }
}

async fn list_storage_directories(path: &Path) -> Result<Vec<StorageDirectoryEntry>, AppError> {
    let mut read_dir = tokio::fs::read_dir(path)
        .await
        .map_err(|error| match error.kind() {
            ErrorKind::PermissionDenied => {
                AppError::ValidationError(format!("目录不可访问: {}", path.display()))
            }
            _ => AppError::IoError(error),
        })?;

    let mut directories = Vec::new();
    while let Some(entry) = read_dir.next_entry().await? {
        let Ok(file_type) = entry.file_type().await else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }

        directories.push(StorageDirectoryEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: path_to_string(&entry.path()),
        });
    }

    directories.sort_by_key(|entry| entry.name.to_ascii_lowercase());
    Ok(directories)
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
