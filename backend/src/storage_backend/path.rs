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

fn shard_hash_from_file_key(file_key: &str) -> Option<&str> {
    let trimmed = file_key.trim();

    if trimmed.len() > 64
        && trimmed.as_bytes().get(64) == Some(&b'.')
        && trimmed[..64]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Some(&trimmed[..64]);
    }

    if let Some(hash) = trimmed
        .strip_prefix("thumb-")
        .and_then(|value| value.get(..64))
        .filter(|value| value.chars().all(|character| character.is_ascii_hexdigit()))
        && trimmed.as_bytes().get(70) == Some(&b'-')
    {
        return Some(hash);
    }

    None
}

pub(super) fn primary_local_path(base: &str, file_key: &str) -> Result<PathBuf, AppError> {
    validate_file_key(file_key)?;
    let mut path = Path::new(base).to_path_buf();
    if let Some(hash) = shard_hash_from_file_key(file_key) {
        path.push(&hash[..2]);
        path.push(&hash[2..4]);
    }
    path.push(file_key);
    Ok(path)
}

pub(super) fn local_path_candidates(base: &str, file_key: &str) -> Result<Vec<PathBuf>, AppError> {
    let primary = primary_local_path(base, file_key)?;
    let legacy = Path::new(base).join(file_key);

    if primary == legacy {
        Ok(vec![primary])
    } else {
        Ok(vec![primary, legacy])
    }
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

    #[test]
    fn primary_local_path_shards_hash_named_originals() {
        let path = primary_local_path(
            "/data/images",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.png",
        )
        .expect("path should build");

        assert_eq!(
            path,
            PathBuf::from(
                "/data/images/01/23/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.png"
            )
        );
    }

    #[test]
    fn primary_local_path_shards_thumbnail_keys_by_underlying_hash() {
        let path = primary_local_path(
            "/data/images",
            "thumb-0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef-300.webp",
        )
        .expect("path should build");

        assert_eq!(
            path,
            PathBuf::from(
                "/data/images/01/23/thumb-0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef-300.webp"
            )
        );
    }

    #[test]
    fn local_path_candidates_keep_legacy_flat_path_for_compatibility() {
        let candidates = local_path_candidates(
            "/data/images",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.png",
        )
        .expect("candidates should build");

        assert_eq!(candidates.len(), 2);
        assert_eq!(
            candidates[0],
            PathBuf::from(
                "/data/images/01/23/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.png"
            )
        );
        assert_eq!(
            candidates[1],
            PathBuf::from(
                "/data/images/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.png"
            )
        );
    }
}
