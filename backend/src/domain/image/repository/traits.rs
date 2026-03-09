use async_trait::async_trait;
use uuid::Uuid;

use crate::models::{Category, Image};

#[async_trait]
#[allow(dead_code)]
pub trait ImageRepository: Send + Sync {
    async fn find_image_by_id(&self, id: Uuid) -> Result<Option<Image>, sqlx::Error>;

    async fn find_images_by_user_paginated(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
        category_id: Option<Uuid>,
        tag: Option<&str>,
    ) -> Result<Vec<Image>, sqlx::Error>;

    async fn count_images_by_user(&self, user_id: Uuid) -> Result<i64, sqlx::Error>;
    async fn create_image(&self, image: &Image) -> Result<(), sqlx::Error>;
    async fn update_image(&self, image: &Image) -> Result<(), sqlx::Error>;
    async fn soft_delete_image(&self, id: Uuid) -> Result<(), sqlx::Error>;
    async fn hard_delete_image(&self, id: Uuid) -> Result<(), sqlx::Error>;
    async fn soft_delete_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error>;
    async fn restore_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error>;
    async fn find_images_by_user_and_ids(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<Vec<Image>, sqlx::Error>;
    async fn find_images_by_user_and_hashes(
        &self,
        user_id: Uuid,
        image_keys: &[String],
    ) -> Result<Vec<Image>, sqlx::Error>;
    async fn hard_delete_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error>;
    async fn find_image_by_hash(
        &self,
        hash: &str,
        user_id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error>;
    async fn find_image_by_hash_global(&self, hash: &str) -> Result<Option<Image>, sqlx::Error>;
    async fn find_deleted_images_by_user(&self, user_id: Uuid) -> Result<Vec<Image>, sqlx::Error>;
    async fn find_deleted_images_by_user_paginated(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<Image>, sqlx::Error>;
    async fn find_images_by_user_cursor(
        &self,
        user_id: Uuid,
        cursor: Option<(chrono::DateTime<chrono::Utc>, Uuid)>,
        limit: i32,
    ) -> Result<Vec<Image>, sqlx::Error>;
}

#[async_trait]
#[allow(dead_code)]
pub trait CategoryRepository: Send + Sync {
    async fn find_categories_by_user(&self, user_id: Uuid) -> Result<Vec<Category>, sqlx::Error>;
    async fn find_category_by_id(&self, id: Uuid) -> Result<Option<Category>, sqlx::Error>;
    async fn create_category(&self, category: &Category) -> Result<(), sqlx::Error>;
    async fn delete_category(&self, id: Uuid) -> Result<(), sqlx::Error>;
}
