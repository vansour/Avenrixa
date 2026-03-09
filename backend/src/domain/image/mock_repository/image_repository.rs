use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::image::repository::ImageRepository;
use crate::models::Image;

pub struct MockImageRepository {
    pub images: Arc<Mutex<Vec<Image>>>,
}

impl MockImageRepository {
    pub fn new() -> Self {
        Self {
            images: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl ImageRepository for MockImageRepository {
    async fn find_image_by_id(&self, id: Uuid) -> Result<Option<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        Ok(images.iter().find(|image| image.id == id).cloned())
    }

    async fn find_images_by_user_paginated(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
        category_id: Option<Uuid>,
        _tag: Option<&str>,
    ) -> Result<Vec<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        let mut filtered: Vec<Image> = images
            .iter()
            .filter(|image| image.user_id == user_id && image.deleted_at.is_none())
            .cloned()
            .collect();

        if let Some(category_id) = category_id {
            filtered.retain(|image| image.category_id == Some(category_id));
        }

        filtered.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });

        let total = filtered.len() as i64;
        let start = offset.max(0) as usize;
        let end = std::cmp::min(start + limit.max(0) as usize, filtered.len());
        if start >= filtered.len() {
            return Ok(Vec::new());
        }

        let mut page = filtered[start..end].to_vec();
        for image in &mut page {
            image.total_count = Some(total);
        }

        Ok(page)
    }

    async fn count_images_by_user(&self, user_id: Uuid) -> Result<i64, sqlx::Error> {
        let images = self.images.lock().unwrap();
        Ok(images
            .iter()
            .filter(|image| image.user_id == user_id && image.deleted_at.is_none())
            .count() as i64)
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

    async fn soft_delete_image(&self, id: Uuid) -> Result<(), sqlx::Error> {
        let mut images = self.images.lock().unwrap();
        if let Some(image) = images.iter_mut().find(|image| image.id == id) {
            image.deleted_at = Some(chrono::Utc::now());
        }
        Ok(())
    }

    async fn hard_delete_image(&self, id: Uuid) -> Result<(), sqlx::Error> {
        let mut images = self.images.lock().unwrap();
        images.retain(|image| image.id != id);
        Ok(())
    }

    async fn soft_delete_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        let mut images = self.images.lock().unwrap();
        let mut affected = 0_u64;

        for image in images.iter_mut() {
            if image.user_id == user_id
                && image_ids.contains(&image.id)
                && image.deleted_at.is_none()
            {
                image.deleted_at = Some(chrono::Utc::now());
                affected += 1;
            }
        }

        Ok(affected)
    }

    async fn restore_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        let mut images = self.images.lock().unwrap();
        let mut affected = 0_u64;

        for image in images.iter_mut() {
            if image.user_id == user_id
                && image_ids.contains(&image.id)
                && image.deleted_at.is_some()
            {
                image.deleted_at = None;
                affected += 1;
            }
        }

        Ok(affected)
    }

    async fn find_images_by_user_and_ids(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<Vec<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        Ok(images
            .iter()
            .filter(|image| image.user_id == user_id && image_ids.contains(&image.id))
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
            .filter(|image| image.user_id == user_id && image_keys.contains(&image.hash))
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
                image.hash == hash && image.user_id == user_id && image.deleted_at.is_none()
            })
            .cloned())
    }

    async fn find_image_by_hash_global(&self, hash: &str) -> Result<Option<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        Ok(images
            .iter()
            .find(|image| image.hash == hash && image.deleted_at.is_none())
            .cloned())
    }

    async fn find_deleted_images_by_user(&self, user_id: Uuid) -> Result<Vec<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        Ok(images
            .iter()
            .filter(|image| image.user_id == user_id && image.deleted_at.is_some())
            .cloned()
            .collect())
    }

    async fn find_deleted_images_by_user_paginated(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        let mut filtered: Vec<Image> = images
            .iter()
            .filter(|image| image.user_id == user_id && image.deleted_at.is_some())
            .cloned()
            .collect();

        filtered.sort_by(|left, right| right.deleted_at.cmp(&left.deleted_at));
        let total = filtered.len() as i64;
        let start = offset.max(0) as usize;
        let end = std::cmp::min(start + limit.max(0) as usize, filtered.len());

        if start >= filtered.len() {
            return Ok(Vec::new());
        }

        let mut page = filtered[start..end].to_vec();
        for image in &mut page {
            image.total_count = Some(total);
        }

        Ok(page)
    }

    async fn find_images_by_user_cursor(
        &self,
        user_id: Uuid,
        cursor: Option<(chrono::DateTime<chrono::Utc>, Uuid)>,
        limit: i32,
    ) -> Result<Vec<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        let mut filtered: Vec<Image> = images
            .iter()
            .filter(|image| image.user_id == user_id && image.deleted_at.is_none())
            .cloned()
            .collect();

        filtered.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });

        let mut result = Vec::new();
        let mut found_cursor = cursor.is_none();

        for image in filtered {
            if !found_cursor
                && let Some((cursor_time, cursor_id)) = cursor
                && (image.created_at < cursor_time
                    || (image.created_at == cursor_time && image.id < cursor_id))
            {
                found_cursor = true;
            }

            if found_cursor {
                result.push(image);
                if result.len() >= limit as usize {
                    break;
                }
            }
        }

        Ok(result)
    }
}
