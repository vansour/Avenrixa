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
        let media_ref_decrements: Vec<(String, i64)> = delete_targets
            .iter()
            .flat_map(|(_, filename, thumbnail, _)| {
                std::iter::once((filename.clone(), -1))
                    .chain(thumbnail.iter().cloned().map(|file_key| (file_key, -1)))
            })
            .collect();
        self.adjust_media_blob_refs(&media_ref_decrements).await?;
        self.cleanup_zero_ref_media_blobs(&all_media_keys).await?;

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
