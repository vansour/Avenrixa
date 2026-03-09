use async_trait::async_trait;
use uuid::Uuid;

use super::{ImageRepository, PostgresImageRepository};
use crate::models::Image;

#[async_trait]
impl ImageRepository for PostgresImageRepository {
    async fn find_image_by_id(&self, id: Uuid) -> Result<Option<Image>, sqlx::Error> {
        self.find_image_by_id_impl(id).await
    }

    async fn find_images_by_user_paginated(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
        category_id: Option<Uuid>,
        tag: Option<&str>,
    ) -> Result<Vec<Image>, sqlx::Error> {
        self.find_images_by_user_paginated_impl(user_id, limit, offset, category_id, tag)
            .await
    }

    async fn count_images_by_user(&self, user_id: Uuid) -> Result<i64, sqlx::Error> {
        self.count_images_by_user_impl(user_id).await
    }

    async fn create_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        self.create_image_impl(image).await
    }

    async fn update_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        self.update_image_impl(image).await
    }

    async fn soft_delete_image(&self, id: Uuid) -> Result<(), sqlx::Error> {
        self.soft_delete_image_impl(id).await
    }

    async fn hard_delete_image(&self, id: Uuid) -> Result<(), sqlx::Error> {
        self.hard_delete_image_impl(id).await
    }

    async fn soft_delete_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        self.soft_delete_images_by_user_impl(user_id, image_ids)
            .await
    }

    async fn restore_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        self.restore_images_by_user_impl(user_id, image_ids).await
    }

    async fn find_images_by_user_and_ids(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<Vec<Image>, sqlx::Error> {
        self.find_images_by_user_and_ids_impl(user_id, image_ids)
            .await
    }

    async fn find_images_by_user_and_hashes(
        &self,
        user_id: Uuid,
        image_keys: &[String],
    ) -> Result<Vec<Image>, sqlx::Error> {
        self.find_images_by_user_and_hashes_impl(user_id, image_keys)
            .await
    }

    async fn hard_delete_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        self.hard_delete_images_by_user_impl(user_id, image_ids)
            .await
    }

    async fn find_image_by_hash(
        &self,
        hash: &str,
        user_id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        self.find_image_by_hash_impl(hash, user_id).await
    }

    async fn find_image_by_hash_global(&self, hash: &str) -> Result<Option<Image>, sqlx::Error> {
        self.find_image_by_hash_global_impl(hash).await
    }

    async fn find_deleted_images_by_user(&self, user_id: Uuid) -> Result<Vec<Image>, sqlx::Error> {
        self.find_deleted_images_by_user_impl(user_id).await
    }

    async fn find_deleted_images_by_user_paginated(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<Image>, sqlx::Error> {
        self.find_deleted_images_by_user_paginated_impl(user_id, limit, offset)
            .await
    }

    async fn find_images_by_user_cursor(
        &self,
        user_id: Uuid,
        cursor: Option<(chrono::DateTime<chrono::Utc>, Uuid)>,
        limit: i32,
    ) -> Result<Vec<Image>, sqlx::Error> {
        self.find_images_by_user_cursor_impl(user_id, cursor, limit)
            .await
    }
}
