use uuid::Uuid;

use super::PostgresImageRepository;
use crate::models::Image;

impl PostgresImageRepository {
    pub(super) async fn create_image_impl(&self, image: &Image) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO images (id, user_id, category_id, filename, thumbnail, original_filename, size, hash, format, views, status, expires_at, deleted_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        )
        .bind(image.id)
        .bind(image.user_id)
        .bind(image.category_id)
        .bind(&image.filename)
        .bind(&image.thumbnail)
        .bind(&image.original_filename)
        .bind(image.size)
        .bind(&image.hash)
        .bind(&image.format)
        .bind(image.views)
        .bind(&image.status)
        .bind(image.expires_at)
        .bind(image.deleted_at)
        .bind(image.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub(super) async fn update_image_impl(&self, image: &Image) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE images
             SET filename = $1,
                 thumbnail = $2,
                 original_filename = $3,
                 category_id = $4,
                 size = $5,
                 hash = $6,
                 format = $7,
                 views = $8,
                 status = $9,
                 expires_at = $10,
                 deleted_at = $11
             WHERE id = $12",
        )
        .bind(&image.filename)
        .bind(&image.thumbnail)
        .bind(&image.original_filename)
        .bind(image.category_id)
        .bind(image.size)
        .bind(&image.hash)
        .bind(&image.format)
        .bind(image.views)
        .bind(&image.status)
        .bind(image.expires_at)
        .bind(image.deleted_at)
        .bind(image.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub(super) async fn soft_delete_image_impl(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE images SET deleted_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub(super) async fn hard_delete_image_impl(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM images WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub(super) async fn soft_delete_images_by_user_impl(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE images
             SET deleted_at = NOW()
             WHERE user_id = $1 AND id = ANY($2) AND deleted_at IS NULL",
        )
        .bind(user_id)
        .bind(image_ids)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub(super) async fn restore_images_by_user_impl(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE images
             SET deleted_at = NULL
             WHERE user_id = $1 AND id = ANY($2) AND deleted_at IS NOT NULL",
        )
        .bind(user_id)
        .bind(image_ids)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub(super) async fn hard_delete_images_by_user_impl(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM images WHERE user_id = $1 AND id = ANY($2)")
            .bind(user_id)
            .bind(image_ids)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}
