use super::*;
use crate::audit::{AuditEvent, record_audit_sync};
use crate::models::ImageStatus;
use std::time::Instant;

impl<I: ImageRepository> ImageDomainService<I> {
    /// 验证文件名安全性
    pub fn validate_filename(&self, filename: &str) -> Result<(), AppError> {
        if filename.is_empty() || filename.len() > 255 {
            return Err(AppError::ValidationError(
                "文件名不能为空且长度不能超过 255".to_string(),
            ));
        }
        if filename.contains("..")
            || filename.contains('/')
            || filename.contains('\\')
            || filename.contains(':')
        {
            warn!("Potentially malicious filename detected: {}", filename);
            return Err(AppError::ValidationError(
                "文件名包含非法路径字符".to_string(),
            ));
        }
        if !filename.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
            return Err(AppError::ValidationError(
                "文件名包含不支持的字符".to_string(),
            ));
        }
        Ok(())
    }

    async fn persist_processed_upload(
        &self,
        user_id: Uuid,
        actor_email: &str,
        ext: String,
        hash: String,
        compressed: Vec<u8>,
        streaming_upload: bool,
    ) -> Result<Image, AppError> {
        let compressed_size = compressed.len() as i64;
        let dedup_strategy = &self.config.image.dedup_strategy;
        let existing_info = self
            .check_image_hash(&hash, dedup_strategy, user_id)
            .await?;

        if let Some(info) = existing_info
            && (dedup_strategy == "user" || info.user_id == user_id)
        {
            info!(
                "Duplicate image detected, returning existing: {} (strategy: {})",
                info.id, dedup_strategy
            );
            let existing_image = self
                .image_repository
                .find_image_by_id(info.id)
                .await?
                .ok_or(AppError::ImageNotFound)?;
            return Ok(existing_image);
        }

        let image_id = Uuid::new_v4();
        let stored_filename = format!("{}.{}", hash, ext);
        let thumbnail_key = self.thumbnail_file_key(&hash);
        let thumbnail_bytes = self.generate_thumbnail_bytes(compressed.clone()).await?;
        let media_ref_adjustments = vec![(stored_filename.clone(), 1), (thumbnail_key.clone(), 1)];
        let media_ref_revert_adjustments =
            vec![(stored_filename.clone(), -1), (thumbnail_key.clone(), -1)];
        let mut wrote_storage_object = false;
        let mut wrote_thumbnail_object = false;
        let mut bumped_media_refs = false;
        if let Err(error) = async {
            self.ensure_media_blob_ready(&stored_filename, "original", Some(&hash))
                .await?;
            self.ensure_media_blob_ready(&thumbnail_key, "thumbnail", Some(&hash))
                .await?;
            wrote_storage_object = self
                .write_storage_object_if_missing(&stored_filename, &compressed)
                .await?;
            wrote_thumbnail_object = self
                .write_storage_object_if_missing(&thumbnail_key, &thumbnail_bytes)
                .await?;
            self.adjust_media_blob_refs(&media_ref_adjustments).await?;
            bumped_media_refs = true;
            Ok::<(), AppError>(())
        }
        .await
        {
            if bumped_media_refs {
                let _ = self
                    .adjust_media_blob_refs(&media_ref_revert_adjustments)
                    .await;
            }
            if wrote_storage_object {
                self.compensate_failed_storage_write(&stored_filename).await;
            }
            if wrote_thumbnail_object {
                self.compensate_orphaned_media(&thumbnail_key).await;
            }
            return Err(error);
        }

        let image = Image {
            id: image_id,
            user_id,
            filename: stored_filename.clone(),
            thumbnail: Some(thumbnail_key.clone()),
            size: compressed_size,
            hash,
            format: ext.clone(),
            views: 0,
            status: ImageStatus::Active,
            expires_at: None,
            created_at: Utc::now(),
            total_count: None,
        };

        if let Err(error) = self.image_repository.create_image(&image).await {
            if bumped_media_refs {
                let _ = self
                    .adjust_media_blob_refs(&media_ref_revert_adjustments)
                    .await;
            }
            if wrote_storage_object {
                self.compensate_failed_storage_write(&stored_filename).await;
            }
            if wrote_thumbnail_object {
                self.compensate_orphaned_media(&thumbnail_key).await;
            }
            return Err(error.into());
        }

        let cache_hint = self.storage_manager.cache_hint(&stored_filename);
        self.cache_image_path(image_id, &cache_hint).await?;

        record_audit_sync(
            &self.database,
            self.observability.as_ref(),
            AuditEvent::new("image.upload", "image")
                .with_user_id(user_id)
                .with_target_id(image_id)
                .with_details(serde_json::json!({
                    "actor_email": actor_email,
                    "stored_filename": stored_filename,
                    "size_bytes": compressed_size,
                    "format": ext,
                    "result": "completed",
                    "risk_level": "info",
                })),
        )
        .await;
        if streaming_upload {
            info!(
                "Image uploaded (streaming): {} by {}",
                image_id, actor_email
            );
        } else {
            info!("Image uploaded: {} by {}", image_id, actor_email);
        }

        Ok(image)
    }

    async fn compensate_failed_storage_write(&self, stored_filename: &str) {
        self.compensate_orphaned_media(stored_filename).await;
    }

    async fn cleanup_temp_file(temp_path: &std::path::Path) {
        if let Err(error) = tokio::fs::remove_file(temp_path).await
            && error.kind() != std::io::ErrorKind::NotFound
        {
            warn!(
                "Failed to remove temporary upload file {}: {}",
                temp_path.display(),
                error
            );
        }
    }

    /// 上传图片（从临时文件）
    #[tracing::instrument(skip(self, temp_path))]
    pub async fn upload_image_from_file(
        &self,
        user_id: Uuid,
        actor_email: &str,
        filename: String,
        temp_path: std::path::PathBuf,
        content_type: Option<String>,
    ) -> Result<Image, AppError> {
        let processing_started_at = Instant::now();
        let upload_result = async {
            self.validate_filename(&filename)?;

            let ext = ImageProcessor::get_extension(&filename);
            if !self.config.storage.allowed_extensions.contains(&ext) {
                warn!("Unsupported extension: {}", ext);
                return Err(AppError::InvalidImageFormat);
            }

            if !content_type
                .as_deref()
                .is_some_and(|ct| ImageProcessor::is_image(Some(ct)))
            {
                warn!("Invalid file type: {:?}", content_type);
                return Err(AppError::InvalidImageFormat);
            }

            let processor = self.image_processor.clone();
            let temp_path_clone = temp_path.clone();
            let ext_clone = ext.clone();
            let compressed = tokio::task::spawn_blocking(move || {
                processor.process_from_file(&temp_path_clone, &ext_clone)
            })
            .await
            .map_err(|e| AppError::Internal(e.into()))??;
            let hash = ImageProcessor::calculate_hash(&compressed);

            self.persist_processed_upload(user_id, actor_email, ext, hash, compressed, true)
                .await
        }
        .await;

        match &upload_result {
            Ok(_) => self
                .observability
                .record_image_processing_success(processing_started_at.elapsed()),
            Err(error) => self.observability.record_image_processing_failure(
                processing_started_at.elapsed(),
                error.to_string(),
            ),
        }

        Self::cleanup_temp_file(&temp_path).await;
        upload_result
    }

    /// 上传图片
    #[cfg(test)]
    #[tracing::instrument(skip(self, data))]
    pub async fn upload_image(
        &self,
        user_id: Uuid,
        actor_email: &str,
        filename: String,
        data: Vec<u8>,
        content_type: Option<String>,
    ) -> Result<Image, AppError> {
        let processing_started_at = Instant::now();
        self.validate_filename(&filename)?;

        let ext = ImageProcessor::get_extension(&filename);
        if !self.config.storage.allowed_extensions.contains(&ext) {
            warn!("Unsupported extension: {}", ext);
            return Err(AppError::InvalidImageFormat);
        }

        if data.len() < 16 {
            return Err(AppError::InvalidImageFormat);
        }
        ImageProcessor::validate_image_bytes(&data)?;

        if !content_type
            .as_deref()
            .is_some_and(|ct| ImageProcessor::is_image(Some(ct)))
        {
            warn!("Invalid file type: {:?}", content_type);
            return Err(AppError::InvalidImageFormat);
        }

        let processor = self.image_processor.clone();
        let ext_clone = ext.clone();
        let compressed = tokio::task::spawn_blocking(move || processor.process(&data, &ext_clone))
            .await
            .map_err(|e| AppError::Internal(e.into()))??;
        let hash = ImageProcessor::calculate_hash(&compressed);

        let upload_result = self
            .persist_processed_upload(user_id, actor_email, ext, hash, compressed, false)
            .await;

        match &upload_result {
            Ok(_) => self
                .observability
                .record_image_processing_success(processing_started_at.elapsed()),
            Err(error) => self.observability.record_image_processing_failure(
                processing_started_at.elapsed(),
                error.to_string(),
            ),
        }

        upload_result
    }
}
