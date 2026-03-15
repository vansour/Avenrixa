use super::*;

impl<I: ImageRepository> ImageDomainService<I> {
    /// 获取图片列表
    #[tracing::instrument(skip(self))]
    pub async fn get_images(
        &self,
        user_id: Uuid,
        page: i32,
        page_size: i32,
    ) -> Result<Paginated<Image>, AppError> {
        let offset = (page - 1) * page_size;

        let cache_key = ImageCache::list(user_id, page, page_size);
        if let Some(manager) = self.cache.as_ref() {
            let mut cache = manager.clone();
            if let Ok(Some(cached)) =
                Cache::get::<Paginated<Image>, _>(&mut cache, &cache_key).await
            {
                return Ok(cached);
            }
        }

        let images = self
            .image_repository
            .find_images_by_user_paginated(user_id, page_size, offset)
            .await?;

        // 提取总数并清理
        let mut images = images;
        let total = images.first().and_then(|img| img.total_count).unwrap_or(0);
        for img in &mut images {
            img.total_count = None;
        }

        // 文件存在性检查逻辑 (如果启用)
        let valid_images = if self.config.storage.enable_file_check {
            let concurrent_threshold = self.config.storage.file_check_concurrent_threshold.max(1);
            let storage_manager = self.storage_manager.clone();

            if images.len() <= concurrent_threshold {
                let mut checked_images = Vec::with_capacity(images.len());
                for img in images {
                    if storage_manager.exists(&img.filename).await.unwrap_or(false) {
                        checked_images.push(img);
                    }
                }
                checked_images
            } else {
                // 并发检查文件存在性后按原索引恢复顺序，避免影响分页排序稳定性
                let mut checked_images = stream::iter(images.into_iter().enumerate())
                    .map(|(idx, img)| {
                        let storage_manager = storage_manager.clone();
                        async move {
                            if storage_manager
                                .exists(img.filename.as_str())
                                .await
                                .unwrap_or(false)
                            {
                                Some((idx, img))
                            } else {
                                None
                            }
                        }
                    })
                    .buffer_unordered(concurrent_threshold)
                    .filter_map(|item| async move { item })
                    .collect::<Vec<_>>()
                    .await;

                checked_images.sort_by_key(|(idx, _)| *idx);
                checked_images.into_iter().map(|(_, img)| img).collect()
            }
        } else {
            images
        };

        let has_next = ((page * page_size) as i64) < total;
        let result = Paginated {
            data: valid_images,
            page,
            page_size,
            total,
            has_next,
        };

        // 缓存结果
        if let Some(manager) = self.cache.as_ref() {
            let mut cache = manager.clone();
            let ttl = self.config.cache_policy.list_ttl;
            let _ = Cache::set(&mut cache, &cache_key, &result, ttl).await;
        }

        Ok(result)
    }

    /// 根据 ID 获取图片
    #[cfg(test)]
    pub async fn get_image_by_id(&self, id: Uuid, user_id: Uuid) -> Result<Image, AppError> {
        let image = self
            .image_repository
            .find_image_by_id(id)
            .await?
            .ok_or(AppError::ImageNotFound)?;

        if image.user_id != user_id {
            return Err(AppError::Forbidden);
        }

        Ok(image)
    }

    /// 根据 image_key（BLAKE3 hash）获取图片
    pub async fn get_image_by_key(
        &self,
        image_key: &str,
        user_id: Uuid,
    ) -> Result<Image, AppError> {
        Self::validate_image_key(image_key)?;
        self.image_repository
            .find_image_by_hash(image_key, user_id)
            .await?
            .ok_or(AppError::ImageNotFound)
    }

    /// 增加图片浏览次数
    #[tracing::instrument(skip(self))]
    pub async fn increment_views(&self, id: Uuid, user_id: Uuid) -> Result<Image, AppError> {
        let mut image = self
            .image_repository
            .find_image_by_id(id)
            .await?
            .ok_or(AppError::ImageNotFound)?;

        if image.user_id != user_id {
            return Err(AppError::Forbidden);
        }

        image.views += 1;
        self.image_repository.update_image(&image).await?;

        // 记录审计日志 (改为异步执行，避免阻塞主流程)
        let database = self.database.clone();
        tokio::spawn(async move {
            log_audit_db(
                &database,
                Some(user_id),
                "image.view",
                "image",
                Some(id),
                None,
                None,
            )
            .await;
        });

        Ok(image)
    }

    pub async fn increment_views_by_key(
        &self,
        image_key: &str,
        user_id: Uuid,
    ) -> Result<Image, AppError> {
        let image = self.get_image_by_key(image_key, user_id).await?;
        self.increment_views(image.id, user_id).await
    }
}
