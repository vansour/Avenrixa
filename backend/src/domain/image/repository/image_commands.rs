use sqlx::QueryBuilder;
use uuid::Uuid;

use super::PostgresImageRepository;

use crate::models::Image;

impl PostgresImageRepository {
    pub(super) async fn create_image(
        &self,
        image: &Image,
    ) -> Result<(), sqlx::Error> {
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

    pub(super) async fn update_image(
        &self,
        image: &Image,
    ) -> Result<(), sqlx::Error> {
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
        let mut builder = QueryBuilder::new("DELETE FROM images WHERE user_id = $1");
        builder.push_bind(user_id);
        for image_id in image_ids {
            builder.push("AND id = ");
            builder.push_bind(image_id);
        }
        let result = builder.build().execute(&self.pool).await?;
        Ok(result.rows_affected())
    }
}
