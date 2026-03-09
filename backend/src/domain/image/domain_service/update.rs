use super::*;

impl<I: ImageRepository, C: CategoryRepository> ImageDomainService<I, C> {
    /// 重命名图片
    pub async fn rename_image(
        &self,
        id: Uuid,
        user_id: Uuid,
        new_filename: &str,
    ) -> Result<(), AppError> {
        if new_filename.is_empty() {
            return Err(AppError::ValidationError("新文件名不能为空".to_string()));
        }

        if let Some(mut img) = self.image_repository.find_image_by_id(id).await?
            && img.user_id == user_id
        {
            img.original_filename = Some(new_filename.to_string());
            self.image_repository.update_image(&img).await?;
            return Ok(());
        }

        Err(AppError::ImageNotFound)
    }

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

    /// 更新图片分类和标签
    pub async fn update_image_category(
        &self,
        id: Uuid,
        user_id: Uuid,
        category_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) -> Result<(), AppError> {
        if let Some(cid) = category_id {
            match self.category_repository.find_category_by_id(cid).await? {
                Some(category) if category.user_id == user_id => {}
                Some(_) => return Err(AppError::Forbidden),
                None => {
                    return Err(AppError::ValidationError("指定的分类不存在".to_string()));
                }
            }
        }

        let normalized_tags = tags.map(Self::normalize_tags).transpose()?;
        let mut tx = self.pool.begin().await?;

        let updated =
            sqlx::query("UPDATE images SET category_id = $1 WHERE id = $2 AND user_id = $3")
                .bind(category_id)
                .bind(id)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;

        if updated.rows_affected() == 0 {
            return Err(AppError::ImageNotFound);
        }

        if let Some(tag_values) = normalized_tags.as_deref() {
            sqlx::query("DELETE FROM image_tags WHERE image_id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;

            if !tag_values.is_empty() {
                let mut builder =
                    QueryBuilder::<Postgres>::new("INSERT INTO image_tags (image_id, tag) ");
                builder.push_values(tag_values.iter(), |mut values, tag| {
                    values.push_bind(id).push_bind(tag);
                });

                builder.build().execute(&mut *tx).await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn update_image_category_by_key(
        &self,
        image_key: &str,
        user_id: Uuid,
        category_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) -> Result<(), AppError> {
        let image = self.get_image_by_key(image_key, user_id).await?;
        self.update_image_category(image.id, user_id, category_id, tags)
            .await
    }
}
