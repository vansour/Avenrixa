use super::*;

impl<I: ImageRepository, C: CategoryRepository> ImageDomainService<I, C> {
    /// 提交文件保存任务
    pub async fn submit_file_save_task(
        &self,
        image_id: Uuid,
        storage_path: String,
        thumbnail_path: String,
        temp_image_path: String,
        thumbnail_data: Vec<u8>,
    ) -> Result<(), AppError> {
        let task_id = Uuid::new_v4().to_string();
        let result_key = self.file_save_queue.result_key_for_task(&task_id);
        let task = FileSaveTask {
            task_id: task_id.clone(),
            image_id: image_id.to_string(),
            storage_path: storage_path.clone(),
            thumbnail_path: thumbnail_path.clone(),
            temp_image_path: temp_image_path.clone(),
            thumbnail_data,
            attempts: 0,
            max_retries: 3,
            result_key,
        };

        let result = self
            .file_save_queue
            .submit_and_wait(task, Duration::from_secs(20))
            .await;

        let result = match result {
            Ok(result) => result,
            Err(e) => {
                // 等待超时后标记取消，防止队列稍后成功造成“有文件无数据库记录”。
                if let Err(cancel_err) = self.file_save_queue.cancel_task(&task_id).await {
                    warn!(
                        "Failed to mark task as cancelled [{}]: {}",
                        image_id, cancel_err
                    );
                }

                // 若任务在超时边界后刚好成功，立即做补偿清理。
                if let Ok(Some(crate::file_queue::FileSaveResult::Success)) =
                    self.file_save_queue.get_task_result(&task_id).await
                {
                    warn!(
                        "Task finished after timeout, cleaning up potential orphan files: {}",
                        image_id
                    );
                    let _ = tokio::fs::remove_file(&storage_path).await;
                    let _ = tokio::fs::remove_file(&thumbnail_path).await;
                }
                // 超时场景先执行一次本地补偿删除，后续由取消标记兜底清理。
                let _ = tokio::fs::remove_file(&storage_path).await;
                let _ = tokio::fs::remove_file(&thumbnail_path).await;

                return Err(AppError::Internal(anyhow::anyhow!(
                    "文件保存队列错误: {}",
                    e
                )));
            }
        };

        match result {
            crate::file_queue::FileSaveResult::Success => {
                info!("File save confirmed for image: {}", image_id);
                Ok(())
            }
            crate::file_queue::FileSaveResult::ImageFailed => {
                let _ = tokio::fs::remove_file(&storage_path).await;
                let _ = tokio::fs::remove_file(&thumbnail_path).await;
                let _ = tokio::fs::remove_file(&temp_image_path).await;
                Err(AppError::Internal(anyhow::anyhow!(
                    "主图片写入失败: {}",
                    image_id
                )))
            }
            crate::file_queue::FileSaveResult::ThumbnailFailed => {
                let _ = tokio::fs::remove_file(&storage_path).await;
                let _ = tokio::fs::remove_file(&thumbnail_path).await;
                let _ = tokio::fs::remove_file(&temp_image_path).await;
                Err(AppError::Internal(anyhow::anyhow!(
                    "缩略图写入失败: {}",
                    image_id
                )))
            }
            crate::file_queue::FileSaveResult::Cancelled => {
                let _ = tokio::fs::remove_file(&storage_path).await;
                let _ = tokio::fs::remove_file(&thumbnail_path).await;
                let _ = tokio::fs::remove_file(&temp_image_path).await;
                Err(AppError::Internal(anyhow::anyhow!(
                    "文件保存任务已取消: {}",
                    image_id
                )))
            }
        }
    }

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
        username: &str,
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
                category_id: None,
                filename: info.filename,
                thumbnail: None,
                original_filename: None,
                size: compressed_size,
                hash,
                format: ext,
                views: 0,
                status: "active".to_string(),
                expires_at: None,
                deleted_at: None,
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
            category_id: None,
            filename: stored_filename.clone(),
            thumbnail: None,
            original_filename: None,
            size: compressed_size,
            hash,
            format: ext,
            views: 0,
            status: "active".to_string(),
            expires_at: None,
            deleted_at: None,
            created_at: Utc::now(),
            total_count: None,
        };

        self.image_repository.create_image(&image).await?;
        let cache_hint = self.storage_manager.cache_hint(&stored_filename);
        self.cache_image_path(image_id, &cache_hint).await?;

        log_audit(
            &self.pool,
            Some(user_id),
            "image.upload",
            "image",
            None,
            None,
            None,
        )
        .await;
        info!("Image uploaded (streaming): {} by {}", image_id, username);

        Ok(image)
    }

    /// 上传图片
    #[tracing::instrument(skip(self, data))]
    pub async fn upload_image(
        &self,
        user_id: Uuid,
        username: &str,
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
        ImageProcessor::validate_image_bytes(&data[..16])?;

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
                category_id: None,
                filename: info.filename,
                thumbnail: None,
                original_filename: None,
                size: compressed_size,
                hash,
                format: ext,
                views: 0,
                status: "active".to_string(),
                expires_at: None,
                deleted_at: None,
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
            category_id: None,
            filename: stored_filename.clone(),
            thumbnail: None,
            original_filename: None,
            size: compressed_size,
            hash,
            format: ext,
            views: 0,
            status: "active".to_string(),
            expires_at: None,
            deleted_at: None,
            created_at: Utc::now(),
            total_count: None,
        };

        self.image_repository.create_image(&image).await?;
        let cache_hint = self.storage_manager.cache_hint(&stored_filename);
        self.cache_image_path(image_id, &cache_hint).await?;

        log_audit(
            &self.pool,
            Some(user_id),
            "image.upload",
            "image",
            None,
            None,
            None,
        )
        .await;
        info!("Image uploaded: {} by {}", image_id, username);

        Ok(image)
    }
}
