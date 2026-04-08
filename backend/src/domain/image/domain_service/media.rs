use super::*;
use crate::storage_backend::enqueue_storage_cleanup_jobs;

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
        let content_type = mime_guess::from_path(&image.filename)
            .first_or_octet_stream()
            .to_string();

        Ok(MediaAsset {
            content_type,
            file_key: image.filename,
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

        Ok(MediaAsset {
            content_type: "image/webp".to_string(),
            file_key: thumbnail_key,
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
                self.ensure_media_blob_ready(&thumbnail_key, "thumbnail", Some(&image.hash))
                    .await?;
                self.adjust_media_blob_refs(&[(thumbnail_key.clone(), 1)])
                    .await?;
                if let Err(error) = self
                    .persist_thumbnail_reference(image, &thumbnail_key)
                    .await
                {
                    let _ = self
                        .adjust_media_blob_refs(&[(thumbnail_key.clone(), -1)])
                        .await;
                    self.compensate_orphaned_media(&thumbnail_key).await;
                    return Err(error);
                }
            }
            return Ok(thumbnail_key);
        }

        let source_image = self.storage_manager.read(&image.filename).await?;
        let thumbnail_bytes = self.generate_thumbnail_bytes(source_image).await?;
        let wrote_thumbnail = self
            .write_storage_object_if_missing(&thumbnail_key, &thumbnail_bytes)
            .await?;
        self.ensure_media_blob_ready(&thumbnail_key, "thumbnail", Some(&image.hash))
            .await?;
        self.adjust_media_blob_refs(&[(thumbnail_key.clone(), 1)])
            .await?;

        if let Err(error) = self
            .persist_thumbnail_reference(image, &thumbnail_key)
            .await
        {
            let _ = self
                .adjust_media_blob_refs(&[(thumbnail_key.clone(), -1)])
                .await;
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

    pub(super) async fn ensure_media_blob_ready(
        &self,
        file_key: &str,
        media_kind: &str,
        content_hash: Option<&str>,
    ) -> Result<(), AppError> {
        self.image_repository
            .upsert_media_blob(file_key, media_kind, content_hash)
            .await?;
        Ok(())
    }

    pub(super) async fn adjust_media_blob_refs(
        &self,
        adjustments: &[(String, i64)],
    ) -> Result<(), AppError> {
        self.image_repository
            .adjust_media_blob_ref_counts(adjustments)
            .await?;
        Ok(())
    }

    pub(super) async fn cleanup_zero_ref_media_blobs(
        &self,
        file_keys: &[String],
    ) -> Result<(), AppError> {
        let unique_keys: Vec<String> = file_keys
            .iter()
            .map(|file_key| file_key.trim())
            .filter(|file_key| !file_key.is_empty())
            .map(ToOwned::to_owned)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        if unique_keys.is_empty() {
            return Ok(());
        }

        let loaded_blobs = self
            .image_repository
            .find_media_blobs_by_keys(&unique_keys)
            .await?;
        let found_blob_keys: HashSet<String> = loaded_blobs
            .iter()
            .map(|blob| blob.storage_key.clone())
            .collect();
        let delete_targets: Vec<String> = loaded_blobs
            .into_iter()
            .filter(|blob| blob.ref_count <= 0 && !blob.is_deleted())
            .map(|blob| blob.storage_key)
            .collect();
        let tracked_targets: HashSet<String> = delete_targets.iter().cloned().collect();
        let mut delete_targets: Vec<String> = delete_targets;
        for file_key in &unique_keys {
            if !found_blob_keys.contains(file_key) {
                delete_targets.push(file_key.clone());
            }
        }
        if delete_targets.is_empty() {
            return Ok(());
        }

        let tracked_delete_targets: Vec<String> = delete_targets
            .iter()
            .filter(|file_key| tracked_targets.contains(*file_key))
            .cloned()
            .collect();
        if !tracked_delete_targets.is_empty() {
            self.image_repository
                .set_media_blob_status(&tracked_delete_targets, "pending_delete")
                .await?;
        }

        let delete_concurrency = self.config.storage.file_check_concurrent_threshold.max(1);
        let storage_manager = self.storage_manager.clone();
        let storage_snapshot = storage_manager.active_settings().storage_settings();
        let failed_targets = if delete_targets.len() <= delete_concurrency {
            let mut failed_targets = Vec::new();
            for file_key in &delete_targets {
                if let Err(error) = storage_manager.delete(file_key).await {
                    warn!(
                        "physical media delete failed for {}: {}. queueing retry",
                        file_key, error
                    );
                    failed_targets.push(file_key.clone());
                }
            }
            failed_targets
        } else {
            stream::iter(delete_targets.iter().cloned())
                .map(|file_key| {
                    let storage_manager = storage_manager.clone();
                    async move {
                        match storage_manager.delete(&file_key).await {
                            Ok(()) => None,
                            Err(error) => {
                                warn!(
                                    "physical media delete failed for {}: {}. queueing retry",
                                    file_key, error
                                );
                                Some(file_key)
                            }
                        }
                    }
                })
                .buffer_unordered(delete_concurrency)
                .filter_map(|item| async move { item })
                .collect::<Vec<_>>()
                .await
        };

        let successful_targets: Vec<String> = delete_targets
            .into_iter()
            .filter(|file_key| !failed_targets.contains(file_key))
            .collect();
        let tracked_successful_targets: Vec<String> = successful_targets
            .iter()
            .filter(|file_key| tracked_targets.contains(*file_key))
            .cloned()
            .collect();
        if !tracked_successful_targets.is_empty() {
            self.image_repository
                .set_media_blob_status(&tracked_successful_targets, "deleted")
                .await?;
        }
        if !failed_targets.is_empty() {
            enqueue_storage_cleanup_jobs(&self.database, &storage_snapshot, &failed_targets)
                .await?;
        }

        Ok(())
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
        if let Err(error) = self
            .cleanup_zero_ref_media_blobs(&[file_key.to_string()])
            .await
        {
            warn!(
                "failed to compensate orphaned media {} after persistence error: {}",
                file_key, error
            );
        }
    }
}
