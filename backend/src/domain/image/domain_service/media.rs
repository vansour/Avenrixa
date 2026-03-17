use super::*;

impl<I: ImageRepository> ImageDomainService<I> {
    pub async fn get_image_by_filename(
        &self,
        filename: &str,
        user_id: Uuid,
    ) -> Result<Image, AppError> {
        self.image_repository
            .find_image_by_filename(filename, user_id)
            .await?
            .ok_or(AppError::ImageNotFound)
    }

    pub async fn load_image_media(
        &self,
        filename: &str,
        user_id: Uuid,
    ) -> Result<MediaAsset, AppError> {
        let image = self.get_image_by_filename(filename, user_id).await?;
        let data = self.storage_manager.read(&image.filename).await?;
        let content_type = mime_guess::from_path(&image.filename)
            .first_or_octet_stream()
            .to_string();

        Ok(MediaAsset {
            content_type,
            data,
            etag: format!("\"{}\"", image.hash),
        })
    }

    pub async fn load_thumbnail_media(
        &self,
        image_key: &str,
        user_id: Uuid,
    ) -> Result<MediaAsset, AppError> {
        let image = self.get_image_by_key(image_key, user_id).await?;
        let thumbnail_key = self.ensure_thumbnail_exists(&image).await?;
        let data = self.storage_manager.read(&thumbnail_key).await?;

        Ok(MediaAsset {
            content_type: "image/webp".to_string(),
            data,
            etag: format!(
                "\"thumb-{}-{}\"",
                image.hash, self.config.image.thumbnail_size
            ),
        })
    }

    pub(super) fn thumbnail_file_key(&self, hash: &str) -> String {
        format!("thumb-{}-{}.webp", hash, self.config.image.thumbnail_size)
    }

    pub(super) async fn generate_thumbnail_bytes(
        &self,
        source_image: Vec<u8>,
    ) -> Result<Vec<u8>, AppError> {
        let processor = self.image_processor.clone();
        let thumbnail_size = self.config.image.thumbnail_size;
        tokio::task::spawn_blocking(move || {
            processor.generate_thumbnail(&source_image, thumbnail_size)
        })
        .await
        .map_err(|error| AppError::Internal(error.into()))?
        .map_err(AppError::Internal)
    }

    pub(super) async fn ensure_thumbnail_exists(&self, image: &Image) -> Result<String, AppError> {
        let thumbnail_key = image
            .thumbnail
            .clone()
            .unwrap_or_else(|| self.thumbnail_file_key(&image.hash));
        if self
            .storage_manager
            .exists(&thumbnail_key)
            .await
            .unwrap_or(false)
        {
            if image.thumbnail.as_deref() != Some(thumbnail_key.as_str()) {
                self.persist_thumbnail_reference(image, &thumbnail_key)
                    .await?;
            }
            return Ok(thumbnail_key);
        }

        let source_image = self.storage_manager.read(&image.filename).await?;
        let thumbnail_bytes = self.generate_thumbnail_bytes(source_image).await?;
        let wrote_thumbnail = self
            .write_storage_object_if_missing(&thumbnail_key, &thumbnail_bytes)
            .await?;

        if let Err(error) = self
            .persist_thumbnail_reference(image, &thumbnail_key)
            .await
        {
            if wrote_thumbnail {
                self.compensate_orphaned_media(&thumbnail_key).await;
            }
            return Err(error);
        }

        Ok(thumbnail_key)
    }

    pub(super) async fn write_storage_object_if_missing(
        &self,
        file_key: &str,
        bytes: &[u8],
    ) -> Result<bool, AppError> {
        if self.storage_manager.exists(file_key).await.unwrap_or(false) {
            return Ok(false);
        }

        self.storage_manager.write(file_key, bytes).await?;
        Ok(true)
    }

    pub(super) async fn persist_thumbnail_reference(
        &self,
        image: &Image,
        thumbnail_key: &str,
    ) -> Result<(), AppError> {
        if image.thumbnail.as_deref() == Some(thumbnail_key) {
            return Ok(());
        }

        let mut updated = image.clone();
        updated.thumbnail = Some(thumbnail_key.to_string());
        self.image_repository.update_image(&updated).await?;
        Ok(())
    }

    pub(super) async fn compensate_orphaned_media(&self, file_key: &str) {
        match self
            .image_repository
            .find_media_keys_still_referenced_excluding_ids(&[file_key.to_string()], &[])
            .await
        {
            Ok(referenced) if referenced.iter().any(|media_key| media_key == file_key) => {}
            Ok(_) => {
                if let Err(error) = self.storage_manager.delete(file_key).await {
                    warn!(
                        "failed to compensate orphaned media {} after persistence error: {}",
                        file_key, error
                    );
                }
            }
            Err(error) => {
                warn!(
                    "failed to check media references before compensation for {}: {}",
                    file_key, error
                );
            }
        }
    }
}
