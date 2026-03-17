use super::*;
use crate::storage_backend::enqueue_storage_cleanup_jobs;
use tracing::warn;

impl<I: ImageRepository> ImageDomainService<I> {
    /// 永久删除图片
    #[tracing::instrument(skip(self))]
    pub async fn hard_delete_images(
        &self,
        image_ids: &[Uuid],
        user_id: Uuid,
    ) -> Result<(), AppError> {
        if image_ids.is_empty() {
            return Ok(());
        }

        let owned_images = self
            .image_repository
            .find_images_by_user_and_ids(user_id, image_ids)
            .await?;
        if owned_images.is_empty() {
            return Ok(());
        }
        let delete_targets: Vec<(Uuid, String, Option<String>, String)> = owned_images
            .into_iter()
            .map(|img| (img.id, img.filename, img.thumbnail, img.hash))
            .collect();
        let owned_ids: Vec<Uuid> = delete_targets.iter().map(|(id, _, _, _)| *id).collect();
        let unique_filenames: Vec<String> = delete_targets
            .iter()
            .map(|(_, filename, _, _)| filename.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let unique_thumbnail_keys: Vec<String> = delete_targets
            .iter()
            .filter_map(|(_, _, thumbnail, _)| thumbnail.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let affected_hashes: Vec<String> = delete_targets
            .iter()
            .map(|(_, _, _, hash)| hash.clone())
            .collect();
        let deleted_count = self
            .image_repository
            .hard_delete_images_by_user(user_id, &owned_ids)
            .await?;
        if deleted_count == 0 {
            return Ok(());
        }

        let all_media_keys: Vec<String> = unique_filenames
            .iter()
            .cloned()
            .chain(unique_thumbnail_keys.iter().cloned())
            .collect();
        let referenced_media_keys: HashSet<String> = self
            .image_repository
            .find_media_keys_still_referenced_excluding_ids(&all_media_keys, &[])
            .await?
            .into_iter()
            .collect();
        let physical_delete_targets: Vec<String> = all_media_keys
            .into_iter()
            .filter(|file_key| !referenced_media_keys.contains(file_key))
            .collect();

        if !physical_delete_targets.is_empty() {
            let delete_concurrency = self.config.storage.file_check_concurrent_threshold.max(1);
            let storage_manager = self.storage_manager.clone();
            let storage_snapshot = storage_manager.active_settings().storage_settings();
            let failed_targets = if physical_delete_targets.len() <= delete_concurrency {
                let mut failed_targets = Vec::new();
                for filename in &physical_delete_targets {
                    if let Err(error) = storage_manager.delete(filename).await {
                        warn!(
                            "physical image delete failed for {}: {}. queueing retry",
                            filename, error
                        );
                        failed_targets.push(filename.clone());
                    }
                }
                failed_targets
            } else {
                stream::iter(physical_delete_targets.iter().cloned())
                    .map(|filename| {
                        let storage_manager = storage_manager.clone();
                        async move {
                            match storage_manager.delete(&filename).await {
                                Ok(()) => None,
                                Err(error) => {
                                    warn!(
                                        "physical image delete failed for {}: {}. queueing retry",
                                        filename, error
                                    );
                                    Some(filename)
                                }
                            }
                        }
                    })
                    .buffer_unordered(delete_concurrency)
                    .filter_map(|item| async move { item })
                    .collect::<Vec<_>>()
                    .await
            };

            if !failed_targets.is_empty() {
                enqueue_storage_cleanup_jobs(&self.database, &storage_snapshot, &failed_targets)
                    .await?;
            }
        }

        self.invalidate_hash_cache_for_user(user_id, &affected_hashes)
            .await?;

        Ok(())
    }

    /// 批量删除图片
    pub async fn delete_images(&self, image_ids: &[Uuid], user_id: Uuid) -> Result<(), AppError> {
        self.hard_delete_images(image_ids, user_id).await?;
        Ok(())
    }

    pub async fn delete_images_by_keys(
        &self,
        image_keys: &[String],
        user_id: Uuid,
    ) -> Result<(), AppError> {
        if image_keys.is_empty() {
            return Ok(());
        }
        for key in image_keys {
            Self::validate_image_key(key)?;
        }

        let owned_images = self
            .image_repository
            .find_images_by_user_and_hashes(user_id, image_keys)
            .await?;
        if owned_images.is_empty() {
            return Ok(());
        }

        let image_ids: Vec<Uuid> = owned_images.into_iter().map(|img| img.id).collect();
        self.delete_images(&image_ids, user_id).await
    }
}
