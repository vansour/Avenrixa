use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::models::{Image, MediaBlob};

#[async_trait]
pub trait ImageRepository: Send + Sync {
    async fn find_image_by_id(&self, id: Uuid) -> Result<Option<Image>, sqlx::Error>;

    async fn find_images_by_user_after_cursor(
        &self,
        user_id: Uuid,
        limit: i32,
        cursor_created_at: Option<DateTime<Utc>>,
        cursor_id: Option<Uuid>,
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
    async fn upsert_media_blob(
        &self,
        storage_key: &str,
        media_kind: &str,
        content_hash: Option<&str>,
    ) -> Result<MediaBlob, sqlx::Error>;
    async fn find_media_blobs_by_keys(
        &self,
        storage_keys: &[String],
    ) -> Result<Vec<MediaBlob>, sqlx::Error>;
    async fn adjust_media_blob_ref_counts(
        &self,
        adjustments: &[(String, i64)],
    ) -> Result<(), sqlx::Error>;
    async fn set_media_blob_status(
        &self,
        storage_keys: &[String],
        status: &str,
    ) -> Result<(), sqlx::Error>;
}
