use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{DatabaseImageRepository, ImageRepository, PostgresImageRepository};
use crate::models::{Image, MediaBlob};

#[async_trait]
impl ImageRepository for PostgresImageRepository {
    async fn find_image_by_id(&self, id: Uuid) -> Result<Option<Image>, sqlx::Error> {
        self.find_image_by_id_impl(id).await
    }

    async fn find_images_by_user_after_cursor(
        &self,
        user_id: Uuid,
        limit: i32,
        cursor_created_at: Option<DateTime<Utc>>,
        cursor_id: Option<Uuid>,
    ) -> Result<Vec<Image>, sqlx::Error> {
        self.find_images_by_user_after_cursor_impl(user_id, limit, cursor_created_at, cursor_id)
            .await
    }

    async fn create_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        self.create_image(image).await
    }

    async fn update_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        self.update_image(image).await
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
        self.hard_delete_images_by_user(user_id, image_ids)
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

    async fn upsert_media_blob(
        &self,
        storage_key: &str,
        media_kind: &str,
        content_hash: Option<&str>,
    ) -> Result<MediaBlob, sqlx::Error> {
        self.upsert_media_blob_impl(storage_key, media_kind, content_hash)
            .await
    }

    async fn find_media_blobs_by_keys(
        &self,
        storage_keys: &[String],
    ) -> Result<Vec<MediaBlob>, sqlx::Error> {
        self.find_media_blobs_by_keys_impl(storage_keys).await
    }

    async fn adjust_media_blob_ref_counts(
        &self,
        adjustments: &[(String, i64)],
    ) -> Result<(), sqlx::Error> {
        self.adjust_media_blob_ref_counts_impl(adjustments).await
    }

    async fn set_media_blob_status(
        &self,
        storage_keys: &[String],
        status: &str,
    ) -> Result<(), sqlx::Error> {
        self.set_media_blob_status_impl(storage_keys, status).await
    }

    async fn find_image_by_filename(
        &self,
        filename: &str,
        user_id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        self.find_image_by_filename_impl(filename, user_id).await
    }
}

#[async_trait]
impl ImageRepository for DatabaseImageRepository {
    async fn find_image_by_id(&self, id: Uuid) -> Result<Option<Image>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.find_image_by_id(id).await,
        }
    }

    async fn find_images_by_user_after_cursor(
        &self,
        user_id: Uuid,
        limit: i32,
        cursor_created_at: Option<DateTime<Utc>>,
        cursor_id: Option<Uuid>,
    ) -> Result<Vec<Image>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => {
                repo.find_images_by_user_after_cursor(user_id, limit, cursor_created_at, cursor_id)
                    .await
            }
        }
    }

    async fn create_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.create_image(image).await,
        }
    }

    async fn update_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.update_image(image).await,
        }
    }

    async fn find_images_by_user_and_ids(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<Vec<Image>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.find_images_by_user_and_ids(user_id, image_ids).await,
        }
    }

    async fn find_images_by_user_and_hashes(
        &self,
        user_id: Uuid,
        image_keys: &[String],
    ) -> Result<Vec<Image>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => {
                repo.find_images_by_user_and_hashes(user_id, image_keys)
                    .await
            }
        }
    }

    async fn hard_delete_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.hard_delete_images_by_user(user_id, image_ids).await,
        }
    }

    async fn find_image_by_hash(
        &self,
        hash: &str,
        user_id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.find_image_by_hash(hash, user_id).await,
        }
    }

    async fn find_image_by_hash_global(&self, hash: &str) -> Result<Option<Image>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.find_image_by_hash_global(hash).await,
        }
    }

    async fn upsert_media_blob(
        &self,
        storage_key: &str,
        media_kind: &str,
        content_hash: Option<&str>,
    ) -> Result<MediaBlob, sqlx::Error> {
        match self {
            Self::Postgres(repo) => {
                repo.upsert_media_blob(storage_key, media_kind, content_hash)
                    .await
            }
        }
    }

    async fn find_media_blobs_by_keys(
        &self,
        storage_keys: &[String],
    ) -> Result<Vec<MediaBlob>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.find_media_blobs_by_keys(storage_keys).await,
        }
    }

    async fn adjust_media_blob_ref_counts(
        &self,
        adjustments: &[(String, i64)],
    ) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.adjust_media_blob_ref_counts(adjustments).await,
        }
    }

    async fn set_media_blob_status(
        &self,
        storage_keys: &[String],
        status: &str,
    ) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.set_media_blob_status(storage_keys, status).await,
        }
    }

    async fn find_image_by_filename(
        &self,
        filename: &str,
        user_id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.find_image_by_filename(filename, user_id).await,
        }
    }
}
