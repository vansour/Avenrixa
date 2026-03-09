use super::*;

impl<I: ImageRepository, C: CategoryRepository> ImageDomainService<I, C> {
    /// 复制图片
    pub async fn duplicate_image_v2(
        &self,
        original_id: Uuid,
        user_id: Uuid,
    ) -> Result<Image, AppError> {
        let (duplicated, _, _) = self.duplicate_image(original_id, user_id).await?;
        Ok(duplicated)
    }

    /// 缓存图片路径到 Redis
    pub async fn cache_image_path(
        &self,
        image_id: Uuid,
        storage_path: &str,
    ) -> Result<(), AppError> {
        if let Some(manager) = self.redis.as_ref() {
            let cache_key = format!("{}{}", self.config.redis.key_prefix, image_id);
            let mut redis = manager.clone();
            let _: Result<(), _> = redis
                .set_ex(cache_key, storage_path, self.config.redis.ttl)
                .await;
        }
        Ok(())
    }

    /// 复制图片
    pub async fn duplicate_image(
        &self,
        original_id: Uuid,
        user_id: Uuid,
    ) -> Result<(Image, String, String), AppError> {
        let original = self.get_image_by_id(original_id, user_id).await?;
        let original_display_name = original
            .original_filename
            .clone()
            .unwrap_or_else(|| original.filename.clone());

        let new_id = Uuid::new_v4();
        let format_ext = original
            .format
            .trim()
            .trim_start_matches('.')
            .to_ascii_lowercase();
        let new_ext = if format_ext.is_empty() {
            ImageProcessor::get_extension(&original.filename)
        } else {
            format_ext
        };
        let new_filename = format!("{}.{}", new_id, new_ext);
        self.storage_manager
            .copy(&original.filename, &new_filename)
            .await?;

        let duplicated = Image {
            id: new_id,
            user_id,
            category_id: original.category_id,
            filename: new_filename,
            thumbnail: None,
            original_filename: Some(format!("copy_of_{}", original_display_name)),
            size: original.size,
            hash: format!("{}-{}", original.hash, new_id),
            format: original.format,
            views: 0,
            status: "active".to_string(),
            expires_at: original.expires_at,
            deleted_at: None,
            created_at: Utc::now(),
            total_count: None,
        };

        let mut tx = self.pool.begin().await?;
        let insert_image = sqlx::query(
            "INSERT INTO images (id, user_id, category_id, filename, thumbnail, original_filename, size, hash, format, views, status, expires_at, deleted_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        )
        .bind(duplicated.id)
        .bind(duplicated.user_id)
        .bind(duplicated.category_id)
        .bind(&duplicated.filename)
        .bind(&duplicated.thumbnail)
        .bind(&duplicated.original_filename)
        .bind(duplicated.size)
        .bind(&duplicated.hash)
        .bind(&duplicated.format)
        .bind(duplicated.views)
        .bind(&duplicated.status)
        .bind(duplicated.expires_at)
        .bind(duplicated.deleted_at)
        .bind(duplicated.created_at)
        .execute(&mut *tx)
        .await;

        if let Err(error) = insert_image {
            let _ = self.storage_manager.delete(&duplicated.filename).await;
            return Err(AppError::DatabaseError(error));
        }

        let copy_tags = sqlx::query(
            "INSERT INTO image_tags (image_id, tag)
             SELECT $1, tag FROM image_tags WHERE image_id = $2",
        )
        .bind(duplicated.id)
        .bind(original_id)
        .execute(&mut *tx)
        .await;

        if let Err(error) = copy_tags {
            let _ = tx.rollback().await;
            let _ = self.storage_manager.delete(&duplicated.filename).await;
            return Err(AppError::DatabaseError(error));
        }

        if let Err(error) = tx.commit().await {
            let _ = self.storage_manager.delete(&duplicated.filename).await;
            return Err(AppError::DatabaseError(error));
        }

        info!(
            "Image duplicated: {} -> {} by user {}",
            original_id, new_id, user_id
        );
        let _ = self.invalidate_hash_cache_patterns_for_user(user_id).await;
        Ok((duplicated, String::new(), String::new()))
    }
}
