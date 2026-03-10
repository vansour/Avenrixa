use super::*;
use sqlx::{Postgres, QueryBuilder, Sqlite};

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

    /// 更新图片标签
    pub async fn update_image_tags(
        &self,
        id: Uuid,
        user_id: Uuid,
        tags: Option<&[String]>,
    ) -> Result<(), AppError> {
        let normalized_tags = tags.map(Self::normalize_tags).transpose()?;
        match &self.database {
            crate::db::DatabasePool::Postgres(pool) => {
                let mut tx = pool.begin().await?;

                let updated = sqlx::query_scalar::<_, i32>(
                    "SELECT 1 FROM images WHERE id = $1 AND user_id = $2",
                )
                .bind(id)
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;

                if updated.is_none() {
                    return Err(AppError::ImageNotFound);
                }

                if let Some(tag_values) = normalized_tags.as_deref() {
                    sqlx::query("DELETE FROM image_tags WHERE image_id = $1")
                        .bind(id)
                        .execute(&mut *tx)
                        .await?;

                    if !tag_values.is_empty() {
                        let mut builder = QueryBuilder::<Postgres>::new(
                            "INSERT INTO image_tags (image_id, tag) ",
                        );
                        builder.push_values(tag_values.iter(), |mut values, tag| {
                            values.push_bind(id).push_bind(tag);
                        });

                        builder.build().execute(&mut *tx).await?;
                    }
                }

                tx.commit().await?;
                Ok(())
            }
            crate::db::DatabasePool::Sqlite(pool) => {
                let mut tx = pool.begin().await?;

                let updated = sqlx::query_scalar::<_, i32>(
                    "SELECT 1 FROM images WHERE id = ?1 AND user_id = ?2",
                )
                .bind(id)
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;

                if updated.is_none() {
                    return Err(AppError::ImageNotFound);
                }

                if let Some(tag_values) = normalized_tags.as_deref() {
                    sqlx::query("DELETE FROM image_tags WHERE image_id = ?1")
                        .bind(id)
                        .execute(&mut *tx)
                        .await?;

                    if !tag_values.is_empty() {
                        let mut builder =
                            QueryBuilder::<Sqlite>::new("INSERT INTO image_tags (image_id, tag) ");
                        builder.push_values(tag_values.iter(), |mut values, tag| {
                            values.push_bind(id).push_bind(tag);
                        });

                        builder.build().execute(&mut *tx).await?;
                    }
                }

                tx.commit().await?;
                Ok(())
            }
        }
    }

    pub async fn update_image_tags_by_key(
        &self,
        image_key: &str,
        user_id: Uuid,
        tags: Option<&[String]>,
    ) -> Result<(), AppError> {
        let image = self.get_image_by_key(image_key, user_id).await?;
        self.update_image_tags(image.id, user_id, tags).await
    }
}
