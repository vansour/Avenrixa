use super::{FileSaveQueue, FileSaveResult, FileSaveTask, staging_path};
use std::path::Path;
use tracing::error;

impl FileSaveQueue {
    pub(super) async fn cleanup_task_artifacts(task: &FileSaveTask) -> std::io::Result<()> {
        let _ = tokio::fs::remove_file(&task.storage_path).await;
        let _ = tokio::fs::remove_file(&task.thumbnail_path).await;
        let _ = tokio::fs::remove_file(&task.temp_image_path).await;

        let image_stage = staging_path(&task.storage_path, &task.task_id, "imgtmp");
        let thumb_stage = staging_path(&task.thumbnail_path, &task.task_id, "thumbtmp");
        let _ = tokio::fs::remove_file(image_stage).await;
        let _ = tokio::fs::remove_file(thumb_stage).await;
        Ok(())
    }

    /// 处理单个文件保存任务
    #[tracing::instrument(skip(task), fields(image_id = %task.image_id))]
    pub(super) async fn process_task(task: FileSaveTask) -> FileSaveResult {
        let image_id = &task.image_id;

        let storage_exists = tokio::fs::try_exists(&task.storage_path)
            .await
            .unwrap_or(false);
        let thumbnail_exists = tokio::fs::try_exists(&task.thumbnail_path)
            .await
            .unwrap_or(false);
        if storage_exists && thumbnail_exists {
            let _ = tokio::fs::remove_file(&task.temp_image_path).await;
            return FileSaveResult::Success;
        }

        if !tokio::fs::try_exists(&task.temp_image_path)
            .await
            .unwrap_or(false)
        {
            error!(
                "临时主图不存在且目标文件不完整，无法继续处理 [{}]: {}",
                image_id, task.temp_image_path
            );
            return FileSaveResult::ImageFailed;
        }

        if let Some(parent) = Path::new(&task.storage_path).parent()
            && let Err(error) = tokio::fs::create_dir_all(parent).await
        {
            error!("创建主图目录失败 [{}]: {}", image_id, error);
            return FileSaveResult::ImageFailed;
        }
        if let Some(parent) = Path::new(&task.thumbnail_path).parent()
            && let Err(error) = tokio::fs::create_dir_all(parent).await
        {
            error!("创建缩略图目录失败 [{}]: {}", image_id, error);
            return FileSaveResult::ThumbnailFailed;
        }

        let image_staging = staging_path(&task.storage_path, &task.task_id, "imgtmp");
        let thumb_staging = staging_path(&task.thumbnail_path, &task.task_id, "thumbtmp");

        let _ = tokio::fs::remove_file(&image_staging).await;
        let _ = tokio::fs::remove_file(&thumb_staging).await;

        if let Err(error) = tokio::fs::copy(&task.temp_image_path, &image_staging).await {
            error!("复制主图到暂存文件失败 [{}]: {}", image_id, error);
            let _ = tokio::fs::remove_file(&image_staging).await;
            return FileSaveResult::ImageFailed;
        }

        if let Err(error) = Self::save_file_with_retry(
            &thumb_staging,
            &task.thumbnail_data,
            u32::from(super::DEFAULT_MAX_RETRIES),
        )
        .await
        {
            error!("保存缩略图暂存文件失败 [{}]: {}", image_id, error);
            let _ = tokio::fs::remove_file(&image_staging).await;
            let _ = tokio::fs::remove_file(&thumb_staging).await;
            return FileSaveResult::ThumbnailFailed;
        }

        if let Err(error) = tokio::fs::rename(&image_staging, &task.storage_path).await {
            error!("原子替换主图失败 [{}]: {}", image_id, error);
            match tokio::fs::copy(&image_staging, &task.storage_path).await {
                Ok(_) => {
                    let _ = tokio::fs::remove_file(&image_staging).await;
                }
                Err(copy_error) => {
                    error!(
                        "主图写入失败（copy fallback） [{}]: {}",
                        image_id, copy_error
                    );
                    let _ = tokio::fs::remove_file(&image_staging).await;
                    let _ = tokio::fs::remove_file(&thumb_staging).await;
                    return FileSaveResult::ImageFailed;
                }
            }
        }

        if let Err(error) = tokio::fs::rename(&thumb_staging, &task.thumbnail_path).await {
            error!("原子替换缩略图失败 [{}]: {}", image_id, error);
            match tokio::fs::copy(&thumb_staging, &task.thumbnail_path).await {
                Ok(_) => {
                    let _ = tokio::fs::remove_file(&thumb_staging).await;
                }
                Err(copy_error) => {
                    error!(
                        "缩略图写入失败（copy fallback） [{}]: {}",
                        image_id, copy_error
                    );
                    let _ = tokio::fs::remove_file(&thumb_staging).await;
                    return FileSaveResult::ThumbnailFailed;
                }
            }
        }

        let _ = tokio::fs::remove_file(&task.temp_image_path).await;
        FileSaveResult::Success
    }

    /// 带重试的文件保存
    pub(super) async fn save_file_with_retry(
        path: &str,
        data: &[u8],
        max_retries: u32,
    ) -> std::io::Result<()> {
        let mut last_error = None;

        for attempt in 1..=max_retries {
            match tokio::fs::write(path, data).await {
                Ok(_) => return Ok(()),
                Err(error) => {
                    last_error = Some(error);
                    if attempt < max_retries {
                        let delay = std::time::Duration::from_millis(100 * (2u64.pow(attempt)));
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| std::io::Error::other("文件写入失败")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn build_task(
        task_id: &str,
        image_id: &str,
        temp_image_path: String,
        storage_path: String,
        thumbnail_path: String,
    ) -> FileSaveTask {
        FileSaveTask {
            task_id: task_id.to_string(),
            image_id: image_id.to_string(),
            storage_path,
            thumbnail_path,
            temp_image_path,
            thumbnail_data: b"thumb-bytes".to_vec(),
            attempts: 0,
            max_retries: 3,
            result_key: None,
        }
    }

    #[tokio::test]
    async fn process_task_persists_image_and_thumbnail() {
        let dir = tempdir().unwrap();
        let temp_image = dir.path().join("upload.tmp");
        let storage_path = dir.path().join("images/final.png");
        let thumbnail_path = dir.path().join("thumbs/final.webp");
        tokio::fs::write(&temp_image, b"image-bytes").await.unwrap();

        let task = build_task(
            "task-success",
            "image-success",
            temp_image.to_string_lossy().into_owned(),
            storage_path.to_string_lossy().into_owned(),
            thumbnail_path.to_string_lossy().into_owned(),
        );

        let result = FileSaveQueue::process_task(task).await;

        assert_eq!(result, FileSaveResult::Success);
        assert_eq!(
            tokio::fs::read(&storage_path).await.unwrap(),
            b"image-bytes"
        );
        assert_eq!(
            tokio::fs::read(&thumbnail_path).await.unwrap(),
            b"thumb-bytes"
        );
        assert!(!tokio::fs::try_exists(&temp_image).await.unwrap());
    }

    #[tokio::test]
    async fn process_task_succeeds_when_target_files_already_exist() {
        let dir = tempdir().unwrap();
        let temp_image = dir.path().join("upload.tmp");
        let storage_path = dir.path().join("images/existing.png");
        let thumbnail_path = dir.path().join("thumbs/existing.webp");

        tokio::fs::create_dir_all(storage_path.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::create_dir_all(thumbnail_path.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&temp_image, b"stale-temp").await.unwrap();
        tokio::fs::write(&storage_path, b"existing-image")
            .await
            .unwrap();
        tokio::fs::write(&thumbnail_path, b"existing-thumb")
            .await
            .unwrap();

        let task = build_task(
            "task-existing",
            "image-existing",
            temp_image.to_string_lossy().into_owned(),
            storage_path.to_string_lossy().into_owned(),
            thumbnail_path.to_string_lossy().into_owned(),
        );

        let result = FileSaveQueue::process_task(task).await;

        assert_eq!(result, FileSaveResult::Success);
        assert_eq!(
            tokio::fs::read(&storage_path).await.unwrap(),
            b"existing-image"
        );
        assert_eq!(
            tokio::fs::read(&thumbnail_path).await.unwrap(),
            b"existing-thumb"
        );
        assert!(!tokio::fs::try_exists(&temp_image).await.unwrap());
    }

    #[tokio::test]
    async fn process_task_rejects_missing_temp_file_when_targets_incomplete() {
        let dir = tempdir().unwrap();
        let temp_image = dir.path().join("missing.tmp");
        let storage_path = dir.path().join("images/final.png");
        let thumbnail_path = dir.path().join("thumbs/final.webp");

        let task = build_task(
            "task-missing-temp",
            "image-missing-temp",
            temp_image.to_string_lossy().into_owned(),
            storage_path.to_string_lossy().into_owned(),
            thumbnail_path.to_string_lossy().into_owned(),
        );

        let result = FileSaveQueue::process_task(task).await;

        assert_eq!(result, FileSaveResult::ImageFailed);
    }

    #[tokio::test]
    async fn cleanup_task_artifacts_removes_final_and_staging_files() {
        let dir = tempdir().unwrap();
        let temp_image = dir.path().join("upload.tmp");
        let storage_path = dir.path().join("images/final.png");
        let thumbnail_path = dir.path().join("thumbs/final.webp");
        let task = build_task(
            "task-cleanup",
            "image-cleanup",
            temp_image.to_string_lossy().into_owned(),
            storage_path.to_string_lossy().into_owned(),
            thumbnail_path.to_string_lossy().into_owned(),
        );

        tokio::fs::create_dir_all(storage_path.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::create_dir_all(thumbnail_path.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&temp_image, b"temp").await.unwrap();
        tokio::fs::write(&storage_path, b"image").await.unwrap();
        tokio::fs::write(&thumbnail_path, b"thumb").await.unwrap();
        tokio::fs::write(
            staging_path(&task.storage_path, &task.task_id, "imgtmp"),
            b"img-stage",
        )
        .await
        .unwrap();
        tokio::fs::write(
            staging_path(&task.thumbnail_path, &task.task_id, "thumbtmp"),
            b"thumb-stage",
        )
        .await
        .unwrap();

        FileSaveQueue::cleanup_task_artifacts(&task).await.unwrap();

        assert!(!tokio::fs::try_exists(&temp_image).await.unwrap());
        assert!(!tokio::fs::try_exists(&storage_path).await.unwrap());
        assert!(!tokio::fs::try_exists(&thumbnail_path).await.unwrap());
        assert!(
            !tokio::fs::try_exists(staging_path(&task.storage_path, &task.task_id, "imgtmp"))
                .await
                .unwrap()
        );
        assert!(
            !tokio::fs::try_exists(staging_path(
                &task.thumbnail_path,
                &task.task_id,
                "thumbtmp"
            ))
            .await
            .unwrap()
        );
    }
}
