use super::*;

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
        let delete_targets: Vec<(Uuid, String, String)> = owned_images
            .into_iter()
            .map(|img| (img.id, img.filename, img.hash))
            .collect();
        let owned_ids: Vec<Uuid> = delete_targets.iter().map(|(id, _, _)| *id).collect();
        let unique_filenames: Vec<String> = delete_targets
            .iter()
            .map(|(_, filename, _)| filename.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let referenced_filenames: HashSet<String> = self
            .image_repository
            .find_filenames_still_referenced_excluding_ids(&unique_filenames, &owned_ids)
            .await?
            .into_iter()
            .collect();

        let delete_concurrency = self.config.storage.file_check_concurrent_threshold.max(1);
        let storage_manager = self.storage_manager.clone();
        let physical_delete_targets: Vec<String> = delete_targets
            .iter()
            .map(|(_, filename, _)| filename.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .filter(|filename| !referenced_filenames.contains(filename))
            .collect();

        if physical_delete_targets.len() <= delete_concurrency {
            for filename in &physical_delete_targets {
                let _ = storage_manager.delete(filename).await;
            }
        } else {
            stream::iter(physical_delete_targets.iter().cloned())
                .map(|filename| {
                    let storage_manager = storage_manager.clone();
                    async move {
                        let _ = storage_manager.delete(&filename).await;
                    }
                })
                .buffer_unordered(delete_concurrency)
                .for_each(|_| async {})
                .await;
        }

        let affected_hashes: Vec<String> = delete_targets
            .iter()
            .map(|(_, _, hash)| hash.clone())
            .collect();
        self.image_repository
            .hard_delete_images_by_user(user_id, &owned_ids)
            .await?;
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
