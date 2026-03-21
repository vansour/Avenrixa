use std::path::{Path, PathBuf};

use crate::error::AppError;

pub(super) fn validate_file_key(file_key: &str) -> Result<(), AppError> {
    let key = file_key.trim();
    if key.is_empty()
        || key.contains('/')
        || key.contains('\\')
        || key.contains("..")
        || key.len() > 255
    {
        return Err(AppError::ValidationError(
            "文件键无效，仅支持单层文件名且长度不能超过 255".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn join_local_path(base: &str, file_key: &str) -> Result<PathBuf, AppError> {
    validate_file_key(file_key)?;
    let path = Path::new(base).join(file_key);
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_file_key_rejects_nested_paths() {
        assert!(matches!(
            validate_file_key("../secret.txt"),
            Err(AppError::ValidationError(message))
                if message.contains("文件键无效")
        ));
        assert!(matches!(
            validate_file_key("nested/file.txt"),
            Err(AppError::ValidationError(message))
                if message.contains("文件键无效")
        ));
    }
}
