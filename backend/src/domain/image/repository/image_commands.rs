use uuid::Uuid;

use super::PostgresImageRepository;
use crate::models::Image;

impl PostgresImageRepository {
    pub(super) async fn create_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO images (id, user_id, filename, thumbnail, size, hash, format, views, status, expires_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(image.id)
        .bind(image.user_id)
        .bind(&image.filename)
        .bind(&image.thumbnail)
        .bind(image.size)
        .bind(&image.hash)
        .bind(&image.format)
        .bind(image.views)
        .bind(image.status.as_str())
        .bind(image.expires_at)
        .bind(image.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn update_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE images
             SET filename = $1,
                 thumbnail = $2,
                 size = $3,
                 hash = $4,
                 format = $5,
                 views = $6,
                 status = $7,
                 expires_at = $8
             WHERE id = $9",
        )
        .bind(&image.filename)
        .bind(&image.thumbnail)
        .bind(image.size)
        .bind(&image.hash)
        .bind(&image.format)
        .bind(image.views)
        .bind(image.status.as_str())
        .bind(image.expires_at)
        .bind(image.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn hard_delete_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        if image_ids.is_empty() {
            return Ok(0);
        }

        let placeholders: String = vec!["?"; image_ids.len()].join(",");

        sqlx::query(&format!(
            "DELETE FROM images WHERE user_id = $1 AND id IN ({})",
            placeholders
        ))
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(image_ids.len() as u64)
    }
}
