use async_trait::async_trait;
use uuid::Uuid;

use crate::models::Image;

#[async_trait]
pub trait ImageRepository: Send + Sync {
    async fn find_image_by_id(&self, id: Uuid) -> Result<Option<Image>, sqlx::Error>;

    async fn find_images_by_user_paginated(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<Image>, sqlx::Error>;
    async fn create_image(&self, image: &Image) -> Result<(), sqlx::Error>;
    async fn update_image(&self, image: &Image) -> Result<(), sqlx::Error>;
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
    async fn find_filenames_still_referenced_excluding_ids(
        &self,
        filenames: &[String],
        excluded_ids: &[Uuid],
    ) -> Result<Vec<String>, sqlx::Error>;
    async fn find_media_keys_still_referenced_excluding_ids(
        &self,
        media_keys: &[String],
        excluded_ids: &[Uuid],
    ) -> Result<Vec<String>, sqlx::Error>;
    async fn find_image_by_filename(
        &self,
        filename: &str,
        user_id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error>;
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
}
