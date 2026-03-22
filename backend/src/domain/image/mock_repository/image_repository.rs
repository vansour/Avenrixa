use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::image::repository::ImageRepository;
use crate::models::{Image, MediaBlob};

pub struct MockImageRepository {
    pub images: Arc<Mutex<Vec<Image>>>,
    pub media_blobs: Arc<Mutex<Vec<MediaBlob>>>,
}

impl MockImageRepository {
    pub fn new() -> Self {
        Self {
            images: Arc::new(Mutex::new(Vec::new())),
            media_blobs: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl ImageRepository for MockImageRepository {
    async fn find_image_by_id(&self, id: Uuid) -> Result<Option<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        Ok(images.iter().find(|image| image.id == id).cloned())
    }

    async fn find_images_by_user_after_cursor(
        &self,
        user_id: Uuid,
        limit: i32,
        cursor_created_at: Option<DateTime<Utc>>,
        cursor_id: Option<Uuid>,
    ) -> Result<Vec<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        let mut filtered: Vec<Image> = images
            .iter()
            .filter(|image| image.user_id == user_id && image.status.is_active())
            .filter(|image| match (cursor_created_at, cursor_id) {
                (Some(cursor_created_at), Some(cursor_id)) => {
                    image.created_at < cursor_created_at
                        || (image.created_at == cursor_created_at && image.id < cursor_id)
                }
                _ => true,
            })
            .cloned()
            .collect();

        filtered.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });

        filtered.truncate(limit.max(0) as usize);
        Ok(filtered)
    }

    async fn create_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        let mut images = self.images.lock().unwrap();
        images.push(image.clone());
        Ok(())
    }

    async fn update_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        let mut images = self.images.lock().unwrap();
        if let Some(index) = images.iter().position(|current| current.id == image.id) {
            images[index] = image.clone();
        }
        Ok(())
    }

    async fn find_images_by_user_and_ids(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<Vec<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        Ok(images
            .iter()
            .filter(|image| {
                image.user_id == user_id
                    && image_ids.contains(&image.id)
                    && image.status.is_active()
            })
            .cloned()
            .collect())
    }

    async fn find_images_by_user_and_hashes(
        &self,
        user_id: Uuid,
        image_keys: &[String],
    ) -> Result<Vec<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        Ok(images
            .iter()
            .filter(|image| {
                image.user_id == user_id
                    && image_keys.contains(&image.hash)
                    && image.status.is_active()
            })
            .cloned()
            .collect())
    }

    async fn hard_delete_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        let mut images = self.images.lock().unwrap();
        let before = images.len();
        images.retain(|image| !(image.user_id == user_id && image_ids.contains(&image.id)));
        Ok((before - images.len()) as u64)
    }

    async fn find_image_by_hash(
        &self,
        hash: &str,
        user_id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        Ok(images
            .iter()
            .find(|image| {
                image.hash == hash && image.user_id == user_id && image.status.is_active()
            })
            .cloned())
    }

    async fn find_image_by_hash_global(&self, hash: &str) -> Result<Option<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        Ok(images
            .iter()
            .find(|image| image.hash == hash && image.status.is_active())
            .cloned())
    }

    async fn upsert_media_blob(
        &self,
        storage_key: &str,
        media_kind: &str,
        content_hash: Option<&str>,
    ) -> Result<MediaBlob, sqlx::Error> {
        let mut media_blobs = self.media_blobs.lock().unwrap();
        let now = Utc::now();

        if let Some(index) = media_blobs
            .iter()
            .position(|blob| blob.storage_key == storage_key)
        {
            let blob = &mut media_blobs[index];
            blob.media_kind = media_kind.to_string();
            if blob.content_hash.is_none() {
                blob.content_hash = content_hash.map(ToOwned::to_owned);
            }
            blob.status = "ready".to_string();
            blob.updated_at = now;
            return Ok(blob.clone());
        }

        let blob = MediaBlob {
            storage_key: storage_key.to_string(),
            media_kind: media_kind.to_string(),
            content_hash: content_hash.map(ToOwned::to_owned),
            ref_count: 0,
            status: "ready".to_string(),
            created_at: now,
            updated_at: now,
        };
        media_blobs.push(blob.clone());
        Ok(blob)
    }

    async fn find_media_blobs_by_keys(
        &self,
        storage_keys: &[String],
    ) -> Result<Vec<MediaBlob>, sqlx::Error> {
        let media_blobs = self.media_blobs.lock().unwrap();
        Ok(media_blobs
            .iter()
            .filter(|blob| storage_keys.contains(&blob.storage_key))
            .cloned()
            .collect())
    }

    async fn adjust_media_blob_ref_counts(
        &self,
        adjustments: &[(String, i64)],
    ) -> Result<(), sqlx::Error> {
        let mut media_blobs = self.media_blobs.lock().unwrap();
        let now = Utc::now();
        let mut aggregated = std::collections::BTreeMap::new();
        for (storage_key, delta) in adjustments {
            *aggregated.entry(storage_key.clone()).or_insert(0_i64) += delta;
        }

        for (storage_key, delta) in aggregated {
            if let Some(blob) = media_blobs
                .iter_mut()
                .find(|blob| blob.storage_key == storage_key)
            {
                blob.ref_count = (blob.ref_count + delta).max(0);
                if blob.ref_count > 0 {
                    blob.status = "ready".to_string();
                }
                blob.updated_at = now;
            }
        }

        Ok(())
    }

    async fn set_media_blob_status(
        &self,
        storage_keys: &[String],
        status: &str,
    ) -> Result<(), sqlx::Error> {
        let mut media_blobs = self.media_blobs.lock().unwrap();
        let now = Utc::now();
        for blob in media_blobs.iter_mut() {
            if storage_keys.contains(&blob.storage_key) {
                blob.status = status.to_string();
                blob.updated_at = now;
            }
        }
        Ok(())
    }

    async fn find_image_by_filename(
        &self,
        filename: &str,
        user_id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        Ok(images
            .iter()
            .find(|image| {
                image.filename == filename && image.user_id == user_id && image.status.is_active()
            })
            .cloned())
    }
}
