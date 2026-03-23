use super::*;
use crate::audit::{AuditEvent, record_audit_best_effort};
use base64::Engine;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ImageListCursorToken {
    created_at_micros: i64,
    id: Uuid,
}

impl<I: ImageRepository> ImageDomainService<I> {
    fn decode_image_list_cursor(cursor: &str) -> Result<ImageListCursorToken, AppError> {
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(cursor)
            .map_err(|_| AppError::ValidationError("图片列表游标无效".to_string()))?;
        let token: ImageListCursorToken = serde_json::from_slice(&raw)
            .map_err(|_| AppError::ValidationError("图片列表游标无效".to_string()))?;
        let Some(_) = chrono::DateTime::<Utc>::from_timestamp_micros(token.created_at_micros)
        else {
            return Err(AppError::ValidationError("图片列表游标无效".to_string()));
        };
        Ok(token)
    }

    fn encode_image_list_cursor(image: &Image) -> Result<String, AppError> {
        let token = ImageListCursorToken {
            created_at_micros: image.created_at.timestamp_micros(),
            id: image.id,
        };
        let bytes = serde_json::to_vec(&token).map_err(|error| AppError::Internal(error.into()))?;
        Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
    }

    /// 获取图片列表
    #[tracing::instrument(skip(self))]
    pub async fn get_images(
        &self,
        user_id: Uuid,
        cursor: Option<&str>,
        limit: i32,
    ) -> Result<CursorPaginated<Image>, AppError> {
        let limit = limit.clamp(1, 100);
        let decoded_cursor = cursor.map(Self::decode_image_list_cursor).transpose()?;

        let cache_key = ImageCache::list(user_id, cursor, limit);
        if let Some(manager) = self.cache.as_ref() {
            let cache = manager.clone();
            if let Ok(Some(cached)) = Cache::get::<CursorPaginated<Image>>(&cache, &cache_key).await
            {
                return Ok(cached);
            }
        }

        let mut images = self
            .image_repository
            .find_images_by_user_after_cursor(
                user_id,
                limit + 1,
                decoded_cursor.as_ref().and_then(|token| {
                    chrono::DateTime::<Utc>::from_timestamp_micros(token.created_at_micros)
                }),
                decoded_cursor.as_ref().map(|token| token.id),
            )
            .await?;

        let has_next = images.len() > limit as usize;
        if has_next {
            images.truncate(limit as usize);
        }

        for image in &mut images {
            image.total_count = None;
        }

        let next_cursor = if has_next {
            images
                .last()
                .map(Self::encode_image_list_cursor)
                .transpose()?
        } else {
            None
        };

        let result = CursorPaginated {
            data: images,
            limit,
            next_cursor,
            has_next,
        };

        // 缓存结果
        if let Some(manager) = self.cache.as_ref() {
            let cache = manager.clone();
            let ttl = self.config.cache_policy.list_ttl;
            let _ = Cache::set(&cache, &cache_key, &result, ttl).await;
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

        record_audit_best_effort(
            self.database.clone(),
            self.observability.clone(),
            AuditEvent::new("image.view", "image")
                .with_user_id(user_id)
                .with_target_id(id),
        );

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
