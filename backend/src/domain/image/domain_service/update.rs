use super::*;

impl<I: ImageRepository> ImageDomainService<I> {
    /// 设置图片过期时间
    pub async fn set_expiry(
        &self,
        id: Uuid,
        user_id: Uuid,
        expires_at: Option<chrono::DateTime<Utc>>,
    ) -> Result<(), AppError> {
        if let Some(mut img) = self.image_repository.find_image_by_id(id).await?
            && img.user_id == user_id
        {
            img.expires_at = expires_at;
            self.image_repository.update_image(&img).await?;
            return Ok(());
        }

        Err(AppError::ImageNotFound)
    }

    pub async fn set_expiry_by_key(
        &self,
        image_key: &str,
        user_id: Uuid,
        expires_at: Option<chrono::DateTime<Utc>>,
    ) -> Result<(), AppError> {
        let image = self.get_image_by_key(image_key, user_id).await?;
        self.set_expiry(image.id, user_id, expires_at).await
    }
}
