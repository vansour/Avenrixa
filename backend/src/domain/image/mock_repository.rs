use async_trait::async_trait;
use uuid::Uuid;
use std::sync::{Arc, Mutex};
use crate::models::{Image, Category};
use super::repository::{ImageRepository, CategoryRepository};

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
        Ok(images.iter().find(|i| i.id == id).cloned())
    }

    async fn find_images_by_user_paginated(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
        _sort_by: &str,
        _sort_order: &str,
        search: Option<&str>,
        category_id: Option<Uuid>,
        _tag: Option<&str>,
    ) -> Result<Vec<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        let mut filtered: Vec<Image> = images.iter()
            .filter(|i| i.user_id == user_id && i.deleted_at.is_none())
            .cloned()
            .collect();

        if let Some(cid) = category_id {
            filtered.retain(|i| i.category_id == Some(cid));
        }

        if let Some(s) = search {
            filtered.retain(|i| i.filename.contains(s));
        }

        let start = offset as usize;
        let end = (offset + limit) as usize;
        let result = filtered.into_iter().skip(start).take(end - start).collect();
        Ok(result)
    }

    async fn count_images_by_user(&self, user_id: Uuid) -> Result<i64, sqlx::Error> {
        let images = self.images.lock().unwrap();
        Ok(images.iter().filter(|i| i.user_id == user_id && i.deleted_at.is_none()).count() as i64)
    }

    async fn create_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        let mut images = self.images.lock().unwrap();
        images.push(image.clone());
        Ok(())
    }

    async fn update_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        let mut images = self.images.lock().unwrap();
        if let Some(idx) = images.iter().position(|i| i.id == image.id) {
            images[idx] = image.clone();
        }
        Ok(())
    }

    async fn soft_delete_image(&self, id: Uuid) -> Result<(), sqlx::Error> {
        let mut images = self.images.lock().unwrap();
        if let Some(i) = images.iter_mut().find(|i| i.id == id) {
            i.deleted_at = Some(chrono::Utc::now());
        }
        Ok(())
    }

    async fn hard_delete_image(&self, id: Uuid) -> Result<(), sqlx::Error> {
        let mut images = self.images.lock().unwrap();
        images.retain(|i| i.id != id);
        Ok(())
    }

    async fn find_image_by_hash(&self, hash: &str, user_id: Uuid) -> Result<Option<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        Ok(images.iter().find(|i| i.hash == hash && i.user_id == user_id && i.deleted_at.is_none()).cloned())
    }

    async fn find_image_by_hash_global(&self, hash: &str) -> Result<Option<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        Ok(images.iter().find(|i| i.hash == hash && i.deleted_at.is_none()).cloned())
    }

    async fn find_deleted_images_by_user(&self, user_id: Uuid) -> Result<Vec<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        Ok(images.iter().filter(|i| i.user_id == user_id && i.deleted_at.is_some()).cloned().collect())
    }

    async fn find_images_by_user_cursor(
        &self,
        user_id: Uuid,
        cursor: Option<(chrono::DateTime<chrono::Utc>, Uuid)>,
        limit: i32,
    ) -> Result<Vec<Image>, sqlx::Error> {
        let images = self.images.lock().unwrap();
        let mut filtered: Vec<Image> = images.iter()
            .filter(|i| i.user_id == user_id && i.deleted_at.is_none())
            .cloned()
            .collect();

        // 模拟按 created_at DESC, id DESC 排序
        filtered.sort_by(|a, b| {
            b.created_at.cmp(&a.created_at).then_with(|| b.id.cmp(&a.id))
        });

        let mut result = Vec::new();
        let mut found_cursor = cursor.is_none();

        for img in filtered {
            if !found_cursor && let Some((c_time, c_id)) = cursor
                && (img.created_at < c_time || (img.created_at == c_time && img.id < c_id)) {
                found_cursor = true;
            }

            if found_cursor {
                result.push(img);
                if result.len() >= limit as usize {
                    break;
                }
            }
        }

        Ok(result)
    }
}

pub struct MockCategoryRepository {
    pub categories: Arc<Mutex<Vec<Category>>>,
}

impl MockCategoryRepository {
    pub fn new() -> Self {
        Self {
            categories: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl CategoryRepository for MockCategoryRepository {
    async fn find_categories_by_user(&self, user_id: Uuid) -> Result<Vec<Category>, sqlx::Error> {
        let categories = self.categories.lock().unwrap();
        Ok(categories.iter().filter(|c| c.user_id == user_id).cloned().collect())
    }

    async fn find_category_by_id(&self, id: Uuid) -> Result<Option<Category>, sqlx::Error> {
        let categories = self.categories.lock().unwrap();
        Ok(categories.iter().find(|c| c.id == id).cloned())
    }

    async fn create_category(&self, category: &Category) -> Result<(), sqlx::Error> {
        let mut categories = self.categories.lock().unwrap();
        categories.push(category.clone());
        Ok(())
    }

    async fn delete_category(&self, id: Uuid) -> Result<(), sqlx::Error> {
        let mut categories = self.categories.lock().unwrap();
        categories.retain(|c| c.id != id);
        Ok(())
    }
}
