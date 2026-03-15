use super::*;
use crate::models::ImageStatus;

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
        self.validate_filename(&filename)?;

        let ext = ImageProcessor::get_extension(&filename);
        if !self.config.storage.allowed_extensions.contains(&ext) {
            warn!("Unsupported extension: {}", ext);
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Err(AppError::InvalidImageFormat);
        }

        if !content_type
            .as_deref()
            .is_some_and(|ct| ImageProcessor::is_image(Some(ct)))
        {
            warn!("Invalid file type: {:?}", content_type);
            let _ = tokio::fs::remove_file(&temp_path).await;
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
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Ok(Image {
                id: info.id,
                user_id,
                filename: info.filename,
                thumbnail: None,
                size: compressed_size,
                hash,
                format: ext.clone(),
                views: 0,
                status: ImageStatus::Active,
                expires_at: None,
                created_at: Utc::now(),
                total_count: None,
            });
        }

        let image_id = Uuid::new_v4();
        let stored_filename = format!("{}.{}", hash, ext);
        if !self
            .storage_manager
            .exists(&stored_filename)
            .await
            .unwrap_or(false)
        {
            self.storage_manager
                .write(&stored_filename, &compressed)
                .await?;
        }
        let _ = tokio::fs::remove_file(&temp_path).await;

        let image = Image {
            id: image_id,
            user_id,
            filename: stored_filename.clone(),
            thumbnail: None,
            size: compressed_size,
            hash,
            format: ext.clone(),
            views: 0,
            status: ImageStatus::Active,
            expires_at: None,
            created_at: Utc::now(),
            total_count: None,
        };

        self.image_repository.create_image(&image).await?;
        let cache_hint = self.storage_manager.cache_hint(&stored_filename);
        self.cache_image_path(image_id, &cache_hint).await?;

        log_audit_db(
            &self.database,
            Some(user_id),
            "image.upload",
            "image",
            Some(image_id),
            None,
            Some(serde_json::json!({
                "actor_email": actor_email,
                "stored_filename": stored_filename,
                "size_bytes": compressed_size,
                "format": ext,
                "result": "completed",
                "risk_level": "info",
            })),
        )
        .await;
        info!(
            "Image uploaded (streaming): {} by {}",
            image_id, actor_email
        );

        Ok(image)
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
            return Ok(Image {
                id: info.id,
                user_id,
                filename: info.filename,
                thumbnail: None,
                size: compressed_size,
                hash,
                format: ext.clone(),
                views: 0,
                status: ImageStatus::Active,
                expires_at: None,
                created_at: Utc::now(),
                total_count: None,
            });
        }

        let image_id = Uuid::new_v4();
        let stored_filename = format!("{}.{}", hash, ext);
        if !self
            .storage_manager
            .exists(&stored_filename)
            .await
            .unwrap_or(false)
        {
            self.storage_manager
                .write(&stored_filename, &compressed)
                .await?;
        }

        let image = Image {
            id: image_id,
            user_id,
            filename: stored_filename.clone(),
            thumbnail: None,
            size: compressed_size,
            hash,
            format: ext.clone(),
            views: 0,
            status: ImageStatus::Active,
            expires_at: None,
            created_at: Utc::now(),
            total_count: None,
        };

        self.image_repository.create_image(&image).await?;
        let cache_hint = self.storage_manager.cache_hint(&stored_filename);
        self.cache_image_path(image_id, &cache_hint).await?;

        log_audit_db(
            &self.database,
            Some(user_id),
            "image.upload",
            "image",
            Some(image_id),
            None,
            Some(serde_json::json!({
                "actor_email": actor_email,
                "stored_filename": stored_filename,
                "size_bytes": compressed_size,
                "format": ext,
                "result": "completed",
                "risk_level": "info",
            })),
        )
        .await;
        info!("Image uploaded: {} by {}", image_id, actor_email);

        Ok(image)
    }
}
