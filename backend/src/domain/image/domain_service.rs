//! 图片领域服务
//!
//! 封装图片相关的业务逻辑

use chrono::Utc;
use futures::stream::{self, StreamExt};
use redis::AsyncCommands;
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::collections::HashSet;
use std::time::Duration;
use uuid::Uuid;

use super::repository::{CategoryRepository, ImageRepository};
use crate::audit::log_audit;
use crate::cache::{Cache, HashCache, ImageCache};
use crate::config::Config;
use crate::error::AppError;
use crate::file_queue::{FileSaveQueue, FileSaveTask};
use crate::image_processor::{FilterParams, ImageProcessor, WatermarkParams};
use crate::models::{Category, EditImageResponse, Image, Paginated};
use crate::storage_backend::StorageManager;
use tracing::{info, warn};

/// 图片哈希检查结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct ImageInfo {
    pub id: Uuid,
    pub filename: String,
    pub user_id: Uuid,
}

const MAX_TAGS_PER_IMAGE: usize = 20;
const MAX_TAG_LENGTH: usize = 50;

/// 图片领域服务
pub struct ImageDomainService<I: ImageRepository, C: CategoryRepository> {
    pool: PgPool,
    redis: Option<redis::aio::ConnectionManager>,
    config: Config,
    image_repository: I,
    category_repository: C,
    image_processor: ImageProcessor,
    file_save_queue: std::sync::Arc<FileSaveQueue>,
    storage_manager: std::sync::Arc<StorageManager>,
}

impl<I: ImageRepository, C: CategoryRepository> ImageDomainService<I, C> {
    fn validate_image_key(image_key: &str) -> Result<(), AppError> {
        if image_key.len() == 64 && image_key.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(());
        }
        Err(AppError::InvalidPagination)
    }

    fn normalize_tags(tags: &[String]) -> Result<Vec<String>, AppError> {
        let mut seen = HashSet::new();
        let mut normalized = Vec::new();

        for raw_tag in tags {
            let tag = raw_tag.trim();
            if tag.is_empty() {
                continue;
            }
            if tag.chars().count() > MAX_TAG_LENGTH || tag.chars().any(char::is_control) {
                return Err(AppError::InvalidPagination);
            }

            let lowered = tag.to_lowercase();
            if seen.insert(lowered.clone()) {
                normalized.push(lowered);
            }
        }

        if normalized.len() > MAX_TAGS_PER_IMAGE {
            return Err(AppError::InvalidPagination);
        }

        Ok(normalized)
    }

    pub fn new(
        pool: PgPool,
        redis: Option<redis::aio::ConnectionManager>,
        config: Config,
        image_repository: I,
        category_repository: C,
        image_processor: ImageProcessor,
        file_save_queue: std::sync::Arc<FileSaveQueue>,
        storage_manager: std::sync::Arc<StorageManager>,
    ) -> Self {
        Self {
            pool,
            redis,
            config,
            image_repository,
            category_repository,
            image_processor,
            file_save_queue,
            storage_manager,
        }
    }

    async fn invalidate_hash_cache_for_user(
        &self,
        user_id: Uuid,
        hashes: &[String],
    ) -> Result<(), AppError> {
        let Some(manager) = self.redis.as_ref() else {
            return Ok(());
        };

        if hashes.is_empty() {
            return Ok(());
        }

        let mut redis = manager.clone();
        let mut unique_hashes = HashSet::new();
        for hash in hashes {
            if hash.trim().is_empty() {
                continue;
            }
            unique_hashes.insert(hash.clone());
        }

        for hash in unique_hashes {
            let _ = Cache::del(&mut redis, HashCache::existing_info(&hash, "user", user_id)).await;
            let _ = Cache::del(&mut redis, HashCache::image_hash(&hash, "user")).await;

            if self.config.image.dedup_strategy == "global" {
                let _ = Cache::del(
                    &mut redis,
                    HashCache::existing_info(&hash, "global", user_id),
                )
                .await;
                let _ = Cache::del(&mut redis, HashCache::image_hash(&hash, "global")).await;
            }
        }

        Ok(())
    }

    async fn invalidate_hash_cache_patterns_for_user(&self, user_id: Uuid) -> Result<(), AppError> {
        let Some(manager) = self.redis.as_ref() else {
            return Ok(());
        };

        let mut redis = manager.clone();
        let _ = Cache::del_pattern(
            &mut redis,
            HashCache::user_existing_info_invalidate(user_id),
        )
        .await;
        let _ = Cache::del_pattern(&mut redis, HashCache::user_hash_invalidate()).await;

        if self.config.image.dedup_strategy == "global" {
            let _ =
                Cache::del_pattern(&mut redis, HashCache::global_existing_info_invalidate()).await;
            let _ = Cache::del_pattern(&mut redis, HashCache::global_hash_invalidate()).await;
        }

        Ok(())
    }

    /// 检查图片 hash 是否已存在（使用缓存）
    #[tracing::instrument(skip(self, hash), fields(hash = %hash))]
    pub async fn check_image_hash(
        &self,
        hash: &str,
        strategy: &str,
        user_id: Uuid,
    ) -> Result<Option<ImageInfo>, AppError> {
        let cache_info_key = HashCache::existing_info(hash, strategy, user_id);

        // 尝试从缓存获取
        if let Some(manager) = self.redis.as_ref() {
            let mut redis = manager.clone();
            if let Ok(Some(cached)) = Cache::get::<ImageInfo, _>(&mut redis, &cache_info_key).await
            {
                info!("Hash cache hit for image hash: {}", hash);
                return Ok(Some(cached));
            }
        }

        // 缓存未命中，查询数据库
        info!(
            "Hash cache miss for image hash: {}, querying database",
            hash
        );
        let existing = match strategy {
            "global" => self
                .image_repository
                .find_image_by_hash_global(hash)
                .await?
                .map(|img| ImageInfo {
                    id: img.id,
                    filename: img.filename,
                    user_id: img.user_id,
                }),
            _ => self
                .image_repository
                .find_image_by_hash(hash, user_id)
                .await?
                .map(|img| ImageInfo {
                    id: img.id,
                    filename: img.filename,
                    user_id: img.user_id,
                }),
        };

        // 缓存结果
        if let (Some(info), Some(manager)) = (&existing, self.redis.as_ref()) {
            let mut redis = manager.clone();
            let cache_ttl = self.config.cache.list_ttl;
            let _ = Cache::set(&mut redis, &cache_info_key, info, cache_ttl).await;
            let hash_cache_key = HashCache::image_hash(hash, strategy);
            let _ = Cache::set(&mut redis, &hash_cache_key, "1", 3600).await;
        }

        Ok(existing)
    }

    /// 获取图片列表
    #[tracing::instrument(skip(self))]
    #[allow(clippy::too_many_arguments)]
    pub async fn get_images(
        &self,
        user_id: Uuid,
        page: i32,
        page_size: i32,
        sort_by: &str,
        sort_order: &str,
        search: Option<&str>,
        category_id: Option<Uuid>,
        tag: Option<&str>,
    ) -> Result<Paginated<Image>, AppError> {
        let offset = (page - 1) * page_size;

        // 尝试从缓存获取 (仅对简单列表查询进行缓存)
        let cache_key =
            ImageCache::list(user_id, page, page_size, category_id, sort_by, sort_order);
        if search.is_none()
            && tag.is_none()
            && let Some(manager) = self.redis.as_ref()
        {
            let mut redis = manager.clone();
            if let Ok(Some(cached)) =
                Cache::get::<Paginated<Image>, _>(&mut redis, &cache_key).await
            {
                return Ok(cached);
            }
        }

        let images = self
            .image_repository
            .find_images_by_user_paginated(
                user_id,
                page_size,
                offset,
                sort_by,
                sort_order,
                search,
                category_id,
                tag,
            )
            .await?;

        // 提取总数并清理
        let mut images = images;
        let total = images.first().and_then(|img| img.total_count).unwrap_or(0);
        for img in &mut images {
            img.total_count = None;
        }

        // 文件存在性检查逻辑 (如果启用)
        let valid_images = if self.config.storage.enable_file_check {
            let concurrent_threshold = self.config.storage.file_check_concurrent_threshold.max(1);
            let storage_manager = self.storage_manager.clone();

            if images.len() <= concurrent_threshold {
                let mut checked_images = Vec::with_capacity(images.len());
                for img in images {
                    if storage_manager.exists(&img.filename).await.unwrap_or(false) {
                        checked_images.push(img);
                    }
                }
                checked_images
            } else {
                // 并发检查文件存在性后按原索引恢复顺序，避免影响分页排序稳定性
                let mut checked_images = stream::iter(images.into_iter().enumerate())
                    .map(|(idx, img)| {
                        let storage_manager = storage_manager.clone();
                        async move {
                            if storage_manager
                                .exists(img.filename.as_str())
                                .await
                                .unwrap_or(false)
                            {
                                Some((idx, img))
                            } else {
                                None
                            }
                        }
                    })
                    .buffer_unordered(concurrent_threshold)
                    .filter_map(|item| async move { item })
                    .collect::<Vec<_>>()
                    .await;

                checked_images.sort_by_key(|(idx, _)| *idx);
                checked_images.into_iter().map(|(_, img)| img).collect()
            }
        } else {
            images
        };

        let has_next = ((page * page_size) as i64) < total;
        let result = Paginated {
            data: valid_images,
            page,
            page_size,
            total,
            has_next,
        };

        // 缓存结果
        if search.is_none()
            && tag.is_none()
            && let Some(manager) = self.redis.as_ref()
        {
            let mut redis = manager.clone();
            let ttl = self.config.cache.list_ttl;
            let _ = Cache::set(&mut redis, &cache_key, &result, ttl).await;
        }

        Ok(result)
    }

    /// 创建图片记录
    pub async fn create_image(&self, image: &Image) -> Result<(), AppError> {
        self.image_repository.create_image(image).await?;
        Ok(())
    }

    /// 根据 ID 获取图片
    pub async fn get_image_by_id(&self, id: Uuid, user_id: Uuid) -> Result<Image, AppError> {
        let image = self
            .image_repository
            .find_image_by_id(id)
            .await?
            .ok_or(AppError::ImageNotFound)?;

        if image.user_id != user_id {
            return Err(AppError::Forbidden);
        }

        Ok(image)
    }

    /// 根据 image_key（BLAKE3 hash）获取图片
    pub async fn get_image_by_key(
        &self,
        image_key: &str,
        user_id: Uuid,
    ) -> Result<Image, AppError> {
        Self::validate_image_key(image_key)?;
        self.image_repository
            .find_image_by_hash(image_key, user_id)
            .await?
            .ok_or(AppError::ImageNotFound)
    }

    /// 增加图片浏览次数
    #[tracing::instrument(skip(self))]
    pub async fn increment_views(&self, id: Uuid, user_id: Uuid) -> Result<Image, AppError> {
        let mut image = self
            .image_repository
            .find_image_by_id(id)
            .await?
            .ok_or(AppError::ImageNotFound)?;

        if image.user_id != user_id {
            return Err(AppError::Forbidden);
        }

        image.views += 1;
        self.image_repository.update_image(&image).await?;

        // 记录审计日志 (改为异步执行，避免阻塞主流程)
        let pool = self.pool.clone();
        tokio::spawn(async move {
            let _ = log_audit(
                &pool,
                Some(user_id),
                "image.view",
                "image",
                Some(id),
                None,
                None,
            )
            .await;
        });

        Ok(image)
    }

    pub async fn increment_views_by_key(
        &self,
        image_key: &str,
        user_id: Uuid,
    ) -> Result<Image, AppError> {
        let image = self.get_image_by_key(image_key, user_id).await?;
        self.increment_views(image.id, user_id).await
    }

    /// 软删除图片
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_images(
        &self,
        image_ids: &[Uuid],
        user_id: Uuid,
    ) -> Result<(), AppError> {
        if image_ids.is_empty() {
            return Ok(());
        }

        let owned_images = self
            .image_repository
            .find_images_by_user_and_ids(user_id, image_ids)
            .await?;
        let affected_hashes: Vec<String> = owned_images.into_iter().map(|img| img.hash).collect();

        self.image_repository
            .soft_delete_images_by_user(user_id, image_ids)
            .await?;
        self.invalidate_hash_cache_for_user(user_id, &affected_hashes)
            .await?;

        Ok(())
    }

    /// 永久删除图片
    #[tracing::instrument(skip(self))]
    pub async fn hard_delete_images(
        &self,
        image_ids: &[Uuid],
        user_id: Uuid,
    ) -> Result<(), AppError> {
        if image_ids.is_empty() {
            return Ok(());
        }

        let owned_images = self
            .image_repository
            .find_images_by_user_and_ids(user_id, image_ids)
            .await?;
        if owned_images.is_empty() {
            return Ok(());
        }
        let delete_targets: Vec<(Uuid, String, String)> = owned_images
            .into_iter()
            .map(|img| (img.id, img.filename, img.hash))
            .collect();

        let delete_concurrency = self.config.storage.file_check_concurrent_threshold.max(1);
        let storage_manager = self.storage_manager.clone();

        if delete_targets.len() <= delete_concurrency {
            for (_, filename, _) in &delete_targets {
                let _ = storage_manager.delete(filename).await;
            }
        } else {
            stream::iter(delete_targets.iter().cloned())
                .map(|(_, filename, _)| {
                    let storage_manager = storage_manager.clone();
                    async move {
                        let _ = storage_manager.delete(&filename).await;
                    }
                })
                .buffer_unordered(delete_concurrency)
                .for_each(|_| async {})
                .await;
        }

        let owned_ids: Vec<Uuid> = delete_targets.iter().map(|(id, _, _)| *id).collect();
        let affected_hashes: Vec<String> = delete_targets
            .iter()
            .map(|(_, _, hash)| hash.clone())
            .collect();
        self.image_repository
            .hard_delete_images_by_user(user_id, &owned_ids)
            .await?;
        self.invalidate_hash_cache_for_user(user_id, &affected_hashes)
            .await?;

        Ok(())
    }

    /// 获取已删除的图片
    pub async fn get_deleted_images(&self, user_id: Uuid) -> Result<Vec<Image>, AppError> {
        let images = self
            .image_repository
            .find_deleted_images_by_user(user_id)
            .await?;
        Ok(images)
    }

    /// 分页获取已删除的图片
    pub async fn get_deleted_images_paginated(
        &self,
        user_id: Uuid,
        page: i32,
        page_size: i32,
    ) -> Result<Paginated<Image>, AppError> {
        if page < 1 || page_size < 1 {
            return Err(AppError::InvalidPagination);
        }

        let offset = (page - 1) * page_size;
        let images = self
            .image_repository
            .find_deleted_images_by_user_paginated(user_id, page_size, offset)
            .await?;

        let mut images = images;
        let total = images.first().and_then(|img| img.total_count).unwrap_or(0);
        for img in &mut images {
            img.total_count = None;
        }

        let has_next = ((page * page_size) as i64) < total;
        Ok(Paginated {
            data: images,
            page,
            page_size,
            total,
            has_next,
        })
    }

    /// 恢复已删除的图片
    pub async fn restore_images(&self, image_ids: &[Uuid], user_id: Uuid) -> Result<(), AppError> {
        if image_ids.is_empty() {
            return Ok(());
        }

        let owned_images = self
            .image_repository
            .find_images_by_user_and_ids(user_id, image_ids)
            .await?;
        let affected_hashes: Vec<String> = owned_images.into_iter().map(|img| img.hash).collect();

        self.image_repository
            .restore_images_by_user(user_id, image_ids)
            .await?;
        self.invalidate_hash_cache_for_user(user_id, &affected_hashes)
            .await?;

        Ok(())
    }

    pub async fn restore_images_by_keys(
        &self,
        image_keys: &[String],
        user_id: Uuid,
    ) -> Result<(), AppError> {
        if image_keys.is_empty() {
            return Ok(());
        }
        for key in image_keys {
            Self::validate_image_key(key)?;
        }

        let owned_images = self
            .image_repository
            .find_images_by_user_and_hashes(user_id, image_keys)
            .await?;
        if owned_images.is_empty() {
            return Ok(());
        }

        let image_ids: Vec<Uuid> = owned_images.into_iter().map(|img| img.id).collect();
        self.restore_images(&image_ids, user_id).await
    }

    /// 重命名图片
    pub async fn rename_image(
        &self,
        id: Uuid,
        user_id: Uuid,
        new_filename: &str,
    ) -> Result<(), AppError> {
        if new_filename.is_empty() {
            return Err(AppError::InvalidPagination);
        }

        if let Some(mut img) = self.image_repository.find_image_by_id(id).await?
            && img.user_id == user_id
        {
            img.original_filename = Some(new_filename.to_string());
            self.image_repository.update_image(&img).await?;
            return Ok(());
        }

        Err(AppError::ImageNotFound)
    }

    /// 设置图片过期时间
    pub async fn set_expiry(
        &self,
        id: Uuid,
        user_id: Uuid,
        expires_at: Option<chrono::DateTime<Utc>>,
    ) -> Result<(), AppError> {
        if let Some(mut img) = self.image_repository.find_image_by_id(id).await?
            && img.user_id == user_id
        {
            img.expires_at = expires_at;
            self.image_repository.update_image(&img).await?;
            return Ok(());
        }

        Err(AppError::ImageNotFound)
    }

    pub async fn set_expiry_by_key(
        &self,
        image_key: &str,
        user_id: Uuid,
        expires_at: Option<chrono::DateTime<Utc>>,
    ) -> Result<(), AppError> {
        let image = self.get_image_by_key(image_key, user_id).await?;
        self.set_expiry(image.id, user_id, expires_at).await
    }

    /// 更新图片分类和标签
    pub async fn update_image_category(
        &self,
        id: Uuid,
        user_id: Uuid,
        category_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) -> Result<(), AppError> {
        if let Some(cid) = category_id {
            match self.category_repository.find_category_by_id(cid).await? {
                Some(category) if category.user_id == user_id => {}
                Some(_) => return Err(AppError::Forbidden),
                None => return Err(AppError::InvalidPagination),
            }
        }

        let normalized_tags = tags.map(Self::normalize_tags).transpose()?;
        let mut tx = self.pool.begin().await?;

        let updated =
            sqlx::query("UPDATE images SET category_id = $1 WHERE id = $2 AND user_id = $3")
                .bind(category_id)
                .bind(id)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;

        if updated.rows_affected() == 0 {
            return Err(AppError::ImageNotFound);
        }

        if let Some(tag_values) = normalized_tags.as_deref() {
            sqlx::query("DELETE FROM image_tags WHERE image_id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;

            if !tag_values.is_empty() {
                let mut builder =
                    QueryBuilder::<Postgres>::new("INSERT INTO image_tags (image_id, tag) ");
                builder.push_values(tag_values.iter(), |mut values, tag| {
                    values.push_bind(id).push_bind(tag);
                });

                builder.build().execute(&mut *tx).await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn update_image_category_by_key(
        &self,
        image_key: &str,
        user_id: Uuid,
        category_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) -> Result<(), AppError> {
        let image = self.get_image_by_key(image_key, user_id).await?;
        self.update_image_category(image.id, user_id, category_id, tags)
            .await
    }

    /// 提交文件保存任务
    pub async fn submit_file_save_task(
        &self,
        image_id: Uuid,
        storage_path: String,
        thumbnail_path: String,
        temp_image_path: String,
        thumbnail_data: Vec<u8>,
    ) -> Result<(), AppError> {
        let task_id = Uuid::new_v4().to_string();
        let result_key = self.file_save_queue.result_key_for_task(&task_id);
        let task = FileSaveTask {
            task_id: task_id.clone(),
            image_id: image_id.to_string(),
            storage_path: storage_path.clone(),
            thumbnail_path: thumbnail_path.clone(),
            temp_image_path: temp_image_path.clone(),
            thumbnail_data,
            attempts: 0,
            max_retries: 3,
            result_key,
        };

        let result = self
            .file_save_queue
            .submit_and_wait(task, Duration::from_secs(20))
            .await;

        let result = match result {
            Ok(result) => result,
            Err(e) => {
                // 等待超时后标记取消，防止队列稍后成功造成“有文件无数据库记录”。
                if let Err(cancel_err) = self.file_save_queue.cancel_task(&task_id).await {
                    warn!(
                        "Failed to mark task as cancelled [{}]: {}",
                        image_id, cancel_err
                    );
                }

                // 若任务在超时边界后刚好成功，立即做补偿清理。
                if let Ok(Some(crate::file_queue::FileSaveResult::Success)) =
                    self.file_save_queue.get_task_result(&task_id).await
                {
                    warn!(
                        "Task finished after timeout, cleaning up potential orphan files: {}",
                        image_id
                    );
                    let _ = tokio::fs::remove_file(&storage_path).await;
                    let _ = tokio::fs::remove_file(&thumbnail_path).await;
                }
                // 超时场景先执行一次本地补偿删除，后续由取消标记兜底清理。
                let _ = tokio::fs::remove_file(&storage_path).await;
                let _ = tokio::fs::remove_file(&thumbnail_path).await;

                return Err(AppError::Internal(anyhow::anyhow!(
                    "文件保存队列错误: {}",
                    e
                )));
            }
        };

        match result {
            crate::file_queue::FileSaveResult::Success => {
                info!("File save confirmed for image: {}", image_id);
                Ok(())
            }
            crate::file_queue::FileSaveResult::ImageFailed => {
                let _ = tokio::fs::remove_file(&storage_path).await;
                let _ = tokio::fs::remove_file(&thumbnail_path).await;
                let _ = tokio::fs::remove_file(&temp_image_path).await;
                Err(AppError::Internal(anyhow::anyhow!(
                    "主图片写入失败: {}",
                    image_id
                )))
            }
            crate::file_queue::FileSaveResult::ThumbnailFailed => {
                let _ = tokio::fs::remove_file(&storage_path).await;
                let _ = tokio::fs::remove_file(&thumbnail_path).await;
                let _ = tokio::fs::remove_file(&temp_image_path).await;
                Err(AppError::Internal(anyhow::anyhow!(
                    "缩略图写入失败: {}",
                    image_id
                )))
            }
            crate::file_queue::FileSaveResult::Cancelled => {
                let _ = tokio::fs::remove_file(&storage_path).await;
                let _ = tokio::fs::remove_file(&thumbnail_path).await;
                let _ = tokio::fs::remove_file(&temp_image_path).await;
                Err(AppError::Internal(anyhow::anyhow!(
                    "文件保存任务已取消: {}",
                    image_id
                )))
            }
        }
    }

    /// 验证文件名安全性
    pub fn validate_filename(&self, filename: &str) -> Result<(), AppError> {
        if filename.is_empty() || filename.len() > 255 {
            return Err(AppError::InvalidPagination);
        }
        if filename.contains("..")
            || filename.contains('/')
            || filename.contains('\\')
            || filename.contains(':')
        {
            warn!("Potentially malicious filename detected: {}", filename);
            return Err(AppError::InvalidPagination);
        }
        if !filename.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
            return Err(AppError::InvalidPagination);
        }
        Ok(())
    }

    /// 上传图片（从临时文件）
    #[tracing::instrument(skip(self, temp_path))]
    pub async fn upload_image_from_file(
        &self,
        user_id: Uuid,
        username: &str,
        filename: String,
        temp_path: std::path::PathBuf,
        content_type: Option<String>,
    ) -> Result<Image, AppError> {
        self.validate_filename(&filename)?;

        let ext = ImageProcessor::get_extension(&filename);
        if !self.config.storage.allowed_extensions.contains(&ext) {
            warn!("Unsupported extension: {}", ext);
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Err(AppError::InvalidImageFormat);
        }

        if !content_type
            .as_deref()
            .is_some_and(|ct| ImageProcessor::is_image(Some(ct)))
        {
            warn!("Invalid file type: {:?}", content_type);
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Err(AppError::InvalidImageFormat);
        }

        let processor = self.image_processor.clone();
        let temp_path_clone = temp_path.clone();
        let ext_clone = ext.clone();
        let compressed = tokio::task::spawn_blocking(move || {
            processor.process_from_file(&temp_path_clone, &ext_clone)
        })
        .await
        .map_err(|e| AppError::Internal(e.into()))??;

        let hash = ImageProcessor::calculate_hash(&compressed);
        let compressed_size = compressed.len() as i64;

        let dedup_strategy = &self.config.image.dedup_strategy;
        let existing_info = self
            .check_image_hash(&hash, dedup_strategy, user_id)
            .await?;

        if let Some(info) = existing_info
            && (dedup_strategy == "user" || info.user_id == user_id)
        {
            info!(
                "Duplicate image detected, returning existing: {} (strategy: {})",
                info.id, dedup_strategy
            );
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Ok(Image {
                id: info.id,
                user_id,
                category_id: None,
                filename: info.filename,
                thumbnail: None,
                original_filename: None,
                size: compressed_size,
                hash,
                format: ext,
                views: 0,
                status: "active".to_string(),
                expires_at: None,
                deleted_at: None,
                created_at: Utc::now(),
                total_count: None,
            });
        }

        let image_id = Uuid::new_v4();
        let stored_filename = format!("{}.{}", hash, ext);
        if !self
            .storage_manager
            .exists(&stored_filename)
            .await
            .unwrap_or(false)
        {
            self.storage_manager
                .write(&stored_filename, &compressed)
                .await?;
        }
        let _ = tokio::fs::remove_file(&temp_path).await;

        let image = Image {
            id: image_id,
            user_id,
            category_id: None,
            filename: stored_filename.clone(),
            thumbnail: None,
            original_filename: None,
            size: compressed_size,
            hash,
            format: ext,
            views: 0,
            status: "active".to_string(),
            expires_at: None,
            deleted_at: None,
            created_at: Utc::now(),
            total_count: None,
        };

        self.image_repository.create_image(&image).await?;
        let cache_hint = self.storage_manager.cache_hint(&stored_filename);
        self.cache_image_path(image_id, &cache_hint).await?;

        log_audit(
            &self.pool,
            Some(user_id),
            "image.upload",
            "image",
            None,
            None,
            None,
        )
        .await;
        info!("Image uploaded (streaming): {} by {}", image_id, username);

        Ok(image)
    }

    /// 上传图片
    #[tracing::instrument(skip(self, data))]
    pub async fn upload_image(
        &self,
        user_id: Uuid,
        username: &str,
        filename: String,
        data: Vec<u8>,
        content_type: Option<String>,
    ) -> Result<Image, AppError> {
        self.validate_filename(&filename)?;

        let ext = ImageProcessor::get_extension(&filename);
        if !self.config.storage.allowed_extensions.contains(&ext) {
            warn!("Unsupported extension: {}", ext);
            return Err(AppError::InvalidImageFormat);
        }

        if data.len() < 16 {
            return Err(AppError::InvalidImageFormat);
        }
        ImageProcessor::validate_image_bytes(&data[..16])?;

        if !content_type
            .as_deref()
            .is_some_and(|ct| ImageProcessor::is_image(Some(ct)))
        {
            warn!("Invalid file type: {:?}", content_type);
            return Err(AppError::InvalidImageFormat);
        }

        let processor = self.image_processor.clone();
        let ext_clone = ext.clone();
        let compressed = tokio::task::spawn_blocking(move || processor.process(&data, &ext_clone))
            .await
            .map_err(|e| AppError::Internal(e.into()))??;

        let hash = ImageProcessor::calculate_hash(&compressed);
        let compressed_size = compressed.len() as i64;

        let dedup_strategy = &self.config.image.dedup_strategy;
        let existing_info = self
            .check_image_hash(&hash, dedup_strategy, user_id)
            .await?;

        if let Some(info) = existing_info
            && (dedup_strategy == "user" || info.user_id == user_id)
        {
            info!(
                "Duplicate image detected, returning existing: {} (strategy: {})",
                info.id, dedup_strategy
            );
            return Ok(Image {
                id: info.id,
                user_id,
                category_id: None,
                filename: info.filename,
                thumbnail: None,
                original_filename: None,
                size: compressed_size,
                hash,
                format: ext,
                views: 0,
                status: "active".to_string(),
                expires_at: None,
                deleted_at: None,
                created_at: Utc::now(),
                total_count: None,
            });
        }

        let image_id = Uuid::new_v4();
        let stored_filename = format!("{}.{}", hash, ext);
        if !self
            .storage_manager
            .exists(&stored_filename)
            .await
            .unwrap_or(false)
        {
            self.storage_manager
                .write(&stored_filename, &compressed)
                .await?;
        }

        let image = Image {
            id: image_id,
            user_id,
            category_id: None,
            filename: stored_filename.clone(),
            thumbnail: None,
            original_filename: None,
            size: compressed_size,
            hash,
            format: ext,
            views: 0,
            status: "active".to_string(),
            expires_at: None,
            deleted_at: None,
            created_at: Utc::now(),
            total_count: None,
        };

        self.image_repository.create_image(&image).await?;
        let cache_hint = self.storage_manager.cache_hint(&stored_filename);
        self.cache_image_path(image_id, &cache_hint).await?;

        log_audit(
            &self.pool,
            Some(user_id),
            "image.upload",
            "image",
            None,
            None,
            None,
        )
        .await;
        info!("Image uploaded: {} by {}", image_id, username);

        Ok(image)
    }

    /// 批量删除图片
    pub async fn delete_images(
        &self,
        image_ids: &[Uuid],
        user_id: Uuid,
        permanent: bool,
    ) -> Result<(), AppError> {
        if permanent {
            self.hard_delete_images(image_ids, user_id).await?;
        } else {
            self.soft_delete_images(image_ids, user_id).await?;
        }
        Ok(())
    }

    pub async fn delete_images_by_keys(
        &self,
        image_keys: &[String],
        user_id: Uuid,
        permanent: bool,
    ) -> Result<(), AppError> {
        if image_keys.is_empty() {
            return Ok(());
        }
        for key in image_keys {
            Self::validate_image_key(key)?;
        }

        let owned_images = self
            .image_repository
            .find_images_by_user_and_hashes(user_id, image_keys)
            .await?;
        if owned_images.is_empty() {
            return Ok(());
        }

        let image_ids: Vec<Uuid> = owned_images.into_iter().map(|img| img.id).collect();
        self.delete_images(&image_ids, user_id, permanent).await
    }

    /// 复制图片
    pub async fn duplicate_image_v2(
        &self,
        original_id: Uuid,
        user_id: Uuid,
    ) -> Result<Image, AppError> {
        let (duplicated, _, _) = self.duplicate_image(original_id, user_id).await?;
        Ok(duplicated)
    }

    /// 编辑图片
    #[tracing::instrument(skip(self, req))]
    pub async fn edit_image(
        &self,
        id: Uuid,
        user_id: Uuid,
        req: crate::models::EditImageRequest,
    ) -> Result<EditImageResponse, AppError> {
        let image = self.get_image_by_id(id, user_id).await?;
        let original_data = self.storage_manager.read(&image.filename).await?;

        let crop = req
            .crop
            .map(|c| (c.x as u32, c.y as u32, c.width as u32, c.height as u32));
        let filters = req.filters.as_ref().map(|f| FilterParams {
            brightness: f.brightness,
            contrast: f.contrast,
            saturation: f.saturation,
            grayscale: f.grayscale,
            sepia: f.sepia,
        });
        let watermark = req.watermark.as_ref().map(|w| WatermarkParams {
            text: w.text.clone(),
            position: w.position.clone(),
            opacity: w.opacity,
        });

        let edited_data = self
            .image_processor
            .edit_image(
                &original_data,
                crop,
                req.rotate,
                &filters,
                &watermark,
                req.convert_format.as_deref(),
            )
            .map_err(|e| AppError::ImageProcessingFailed { source: e })?;

        self.storage_manager
            .write(&image.filename, &edited_data)
            .await?;

        log_audit(
            &self.pool,
            Some(user_id),
            "image.edit",
            "image",
            None,
            None,
            None,
        )
        .await;

        Ok(EditImageResponse {
            image_key: image.hash.clone(),
            edited_url: format!("/images/{}", image.filename),
            thumbnail_url: format!("/thumbnails/{}.webp", image.hash),
        })
    }

    pub async fn edit_image_by_key(
        &self,
        image_key: &str,
        user_id: Uuid,
        req: crate::models::EditImageRequest,
    ) -> Result<EditImageResponse, AppError> {
        let image = self.get_image_by_key(image_key, user_id).await?;
        self.edit_image(image.id, user_id, req).await
    }

    /// 缓存图片路径到 Redis
    pub async fn cache_image_path(
        &self,
        image_id: Uuid,
        storage_path: &str,
    ) -> Result<(), AppError> {
        if let Some(manager) = self.redis.as_ref() {
            let cache_key = format!("{}{}", self.config.redis.key_prefix, image_id);
            let mut redis = manager.clone();
            let _: Result<(), _> = redis
                .set_ex(cache_key, storage_path, self.config.redis.ttl)
                .await;
        }
        Ok(())
    }

    /// 复制图片
    pub async fn duplicate_image(
        &self,
        original_id: Uuid,
        user_id: Uuid,
    ) -> Result<(Image, String, String), AppError> {
        // 获取原图
        let original = self.get_image_by_id(original_id, user_id).await?;
        let original_display_name = original
            .original_filename
            .clone()
            .unwrap_or_else(|| original.filename.clone());

        let new_id = Uuid::new_v4();
        let format_ext = original
            .format
            .trim()
            .trim_start_matches('.')
            .to_ascii_lowercase();
        let new_ext = if format_ext.is_empty() {
            ImageProcessor::get_extension(&original.filename)
        } else {
            format_ext
        };
        let new_filename = format!("{}.{}", new_id, new_ext);
        self.storage_manager
            .copy(&original.filename, &new_filename)
            .await?;

        let duplicated = Image {
            id: new_id,
            user_id,
            category_id: original.category_id,
            filename: new_filename,
            thumbnail: None,
            original_filename: Some(format!("copy_of_{}", original_display_name)),
            size: original.size,
            hash: format!("{}-{}", original.hash, new_id),
            format: original.format,
            views: 0,
            status: "active".to_string(),
            expires_at: original.expires_at,
            deleted_at: None,
            created_at: Utc::now(),
            total_count: None,
        };

        let mut tx = self.pool.begin().await?;
        let insert_image = sqlx::query(
            "INSERT INTO images (id, user_id, category_id, filename, thumbnail, original_filename, size, hash, format, views, status, expires_at, deleted_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        )
        .bind(duplicated.id)
        .bind(duplicated.user_id)
        .bind(duplicated.category_id)
        .bind(&duplicated.filename)
        .bind(&duplicated.thumbnail)
        .bind(&duplicated.original_filename)
        .bind(duplicated.size)
        .bind(&duplicated.hash)
        .bind(&duplicated.format)
        .bind(duplicated.views)
        .bind(&duplicated.status)
        .bind(duplicated.expires_at)
        .bind(duplicated.deleted_at)
        .bind(duplicated.created_at)
        .execute(&mut *tx)
        .await;

        if let Err(e) = insert_image {
            let _ = self.storage_manager.delete(&duplicated.filename).await;
            return Err(AppError::DatabaseError(e));
        }

        let copy_tags = sqlx::query(
            "INSERT INTO image_tags (image_id, tag)
             SELECT $1, tag FROM image_tags WHERE image_id = $2",
        )
        .bind(duplicated.id)
        .bind(original_id)
        .execute(&mut *tx)
        .await;

        if let Err(e) = copy_tags {
            let _ = tx.rollback().await;
            let _ = self.storage_manager.delete(&duplicated.filename).await;
            return Err(AppError::DatabaseError(e));
        }

        if let Err(e) = tx.commit().await {
            let _ = self.storage_manager.delete(&duplicated.filename).await;
            return Err(AppError::DatabaseError(e));
        }

        info!(
            "Image duplicated: {} -> {} by user {}",
            original_id, new_id, user_id
        );
        let _ = self.invalidate_hash_cache_patterns_for_user(user_id).await;
        Ok((duplicated, String::new(), String::new()))
    }

    /// 获取分类列表
    pub async fn get_categories(&self, user_id: Uuid) -> Result<Vec<Category>, AppError> {
        let categories = self
            .category_repository
            .find_categories_by_user(user_id)
            .await?;
        Ok(categories)
    }

    /// 创建分类
    pub async fn create_category(&self, category: &Category) -> Result<(), AppError> {
        self.category_repository.create_category(category).await?;
        Ok(())
    }

    /// 删除分类
    pub async fn delete_category(&self, id: Uuid) -> Result<(), AppError> {
        self.category_repository.delete_category(id).await?;
        Ok(())
    }

    /// 获取配置引用
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 获取图片处理器引用
    /// 获取图片处理器引用
    pub fn image_processor(&self) -> &ImageProcessor {
        &self.image_processor
    }

    /// 获取 Redis 连接引用
    pub fn redis(&self) -> Option<redis::aio::ConnectionManager> {
        self.redis.clone()
    }

    /// Cursor-based 图片分页
    #[tracing::instrument(skip(self))]
    pub async fn get_images_cursor(
        &self,
        user_id: Uuid,
        params: crate::models::PaginationParams,
    ) -> Result<crate::models::CursorPaginated<Image>, AppError> {
        let limit = params.page_size.unwrap_or(20).clamp(1, 100);

        let cursor = match (params.cursor_created_at, params.cursor_id, params.cursor) {
            (Some(time), Some(id), _) => Some((time, id)),
            (Some(_), None, _) | (None, Some(_), _) => return Err(AppError::InvalidPagination),
            (_, _, Some((time, id_str))) => {
                let id = Uuid::parse_str(&id_str).map_err(|_| AppError::InvalidPagination)?;
                Some((time, id))
            }
            _ => None,
        };

        let images = self
            .image_repository
            .find_images_by_user_cursor(user_id, cursor, limit)
            .await?;

        let next_cursor = if images.len() == limit as usize {
            images
                .last()
                .map(|img| (img.created_at, img.id.to_string()))
        } else {
            None
        };

        Ok(crate::models::CursorPaginated {
            data: images,
            next_cursor,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::domain::image::mock_repository::{MockCategoryRepository, MockImageRepository};
    use crate::file_queue::FileSaveQueue;
    use crate::image_processor::ImageProcessor;
    use crate::runtime_settings::RuntimeSettingsService;
    use crate::storage_backend::StorageManager;
    use std::sync::Arc;

    async fn setup_service() -> ImageDomainService<MockImageRepository, MockCategoryRepository> {
        let config = Config::default();
        let image_processor = ImageProcessor::new(1920, 1080, 200, 80);

        // 在测试中，如果不连接 Redis，直接传 None
        let redis = None;

        let file_save_queue = Arc::new(FileSaveQueue::new_mock());

        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").unwrap();
        let runtime_settings = Arc::new(RuntimeSettingsService::new(pool.clone(), &config));
        let storage_manager = Arc::new(StorageManager::new(runtime_settings));

        ImageDomainService::new(
            pool,
            redis,
            config,
            MockImageRepository::new(),
            MockCategoryRepository::new(),
            image_processor,
            file_save_queue,
            storage_manager,
        )
    }

    fn build_image(
        id: Uuid,
        user_id: Uuid,
        filename: &str,
        hash: &str,
        created_at: chrono::DateTime<Utc>,
        deleted_at: Option<chrono::DateTime<Utc>>,
    ) -> Image {
        Image {
            id,
            user_id,
            category_id: None,
            filename: filename.to_string(),
            thumbnail: None,
            original_filename: None,
            size: 100,
            hash: hash.to_string(),
            format: "jpg".to_string(),
            views: 0,
            status: "active".to_string(),
            expires_at: None,
            deleted_at,
            created_at,
            total_count: None,
        }
    }

    #[tokio::test]
    async fn test_get_image_not_found() {
        let service = setup_service().await;
        let user_id = Uuid::new_v4();
        let image_id = Uuid::new_v4();
        let result = service.get_image_by_id(image_id, user_id).await;
        assert!(matches!(result, Err(AppError::ImageNotFound)));
    }

    #[tokio::test]
    async fn test_increment_views() {
        let service = setup_service().await;
        let user_id = Uuid::new_v4();
        let image_id = Uuid::new_v4();

        let image = Image {
            id: image_id,
            user_id,
            category_id: None,
            filename: "test.jpg".to_string(),
            thumbnail: None,
            original_filename: None,
            size: 100,
            hash: "hash".to_string(),
            format: "jpg".to_string(),
            views: 0,
            status: "active".to_string(),
            expires_at: None,
            deleted_at: None,
            created_at: Utc::now(),
            total_count: None,
        };

        service.image_repository.create_image(&image).await.unwrap();

        let updated = service.increment_views(image_id, user_id).await.unwrap();
        assert_eq!(updated.views, 1);

        let fetched = service.get_image_by_id(image_id, user_id).await.unwrap();
        assert_eq!(fetched.views, 1);
    }

    #[tokio::test]
    async fn test_rename_image() {
        let service = setup_service().await;
        let user_id = Uuid::new_v4();
        let image_id = Uuid::new_v4();

        let image = Image {
            id: image_id,
            user_id,
            category_id: None,
            filename: "test.jpg".to_string(),
            thumbnail: None,
            original_filename: Some("old.jpg".to_string()),
            size: 100,
            hash: "hash".to_string(),
            format: "jpg".to_string(),
            views: 0,
            status: "active".to_string(),
            expires_at: None,
            deleted_at: None,
            created_at: Utc::now(),
            total_count: None,
        };

        service.image_repository.create_image(&image).await.unwrap();

        service
            .rename_image(image_id, user_id, "new.jpg")
            .await
            .unwrap();

        let fetched = service.get_image_by_id(image_id, user_id).await.unwrap();
        assert_eq!(fetched.original_filename, Some("new.jpg".to_string()));
    }

    #[tokio::test]
    async fn test_soft_delete_images() {
        let service = setup_service().await;
        let user_id = Uuid::new_v4();
        let image_id = Uuid::new_v4();

        let image = Image {
            id: image_id,
            user_id,
            category_id: None,
            filename: "test.jpg".to_string(),
            thumbnail: None,
            original_filename: None,
            size: 100,
            hash: "hash".to_string(),
            format: "jpg".to_string(),
            views: 0,
            status: "active".to_string(),
            expires_at: None,
            deleted_at: None,
            created_at: Utc::now(),
            total_count: None,
        };

        service.image_repository.create_image(&image).await.unwrap();

        service
            .soft_delete_images(&[image_id], user_id)
            .await
            .unwrap();

        let fetched = service
            .image_repository
            .find_image_by_id(image_id)
            .await
            .unwrap()
            .unwrap();
        assert!(fetched.deleted_at.is_some());
    }

    #[tokio::test]
    async fn test_get_images_cursor_rejects_partial_cursor_pair() {
        let service = setup_service().await;
        let user_id = Uuid::new_v4();

        let params_only_time = crate::models::PaginationParams {
            page: None,
            page_size: Some(20),
            sort_by: None,
            sort_order: None,
            search: None,
            category_id: None,
            tag: None,
            cursor_created_at: Some(Utc::now()),
            cursor_id: None,
            cursor: None,
        };
        let result = service.get_images_cursor(user_id, params_only_time).await;
        assert!(matches!(result, Err(AppError::InvalidPagination)));

        let params_only_id = crate::models::PaginationParams {
            page: None,
            page_size: Some(20),
            sort_by: None,
            sort_order: None,
            search: None,
            category_id: None,
            tag: None,
            cursor_created_at: None,
            cursor_id: Some(Uuid::new_v4()),
            cursor: None,
        };
        let result = service.get_images_cursor(user_id, params_only_id).await;
        assert!(matches!(result, Err(AppError::InvalidPagination)));
    }

    #[tokio::test]
    async fn test_get_images_cursor_accepts_new_cursor_pair() {
        let service = setup_service().await;
        let user_id = Uuid::new_v4();

        let cursor_image_id = Uuid::new_v4();
        let older_image_id = Uuid::new_v4();
        let cursor_time = Utc::now();
        let older_time = cursor_time - chrono::Duration::seconds(1);

        let cursor_image = build_image(
            cursor_image_id,
            user_id,
            "cursor.jpg",
            "hash-cursor",
            cursor_time,
            None,
        );
        let older_image = build_image(
            older_image_id,
            user_id,
            "older.jpg",
            "hash-older",
            older_time,
            None,
        );

        service
            .image_repository
            .create_image(&cursor_image)
            .await
            .unwrap();
        service
            .image_repository
            .create_image(&older_image)
            .await
            .unwrap();

        let params = crate::models::PaginationParams {
            page: None,
            page_size: Some(20),
            sort_by: None,
            sort_order: None,
            search: None,
            category_id: None,
            tag: None,
            cursor_created_at: Some(cursor_time),
            cursor_id: Some(cursor_image_id),
            cursor: None,
        };

        let result = service.get_images_cursor(user_id, params).await.unwrap();
        assert_eq!(result.data.len(), 1);
        assert_eq!(result.data[0].id, older_image_id);
    }

    #[tokio::test]
    async fn test_get_deleted_images_paginated() {
        let service = setup_service().await;
        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let deleted_1 = build_image(
            Uuid::new_v4(),
            user_id,
            "deleted-1.jpg",
            "hash-1",
            now - chrono::Duration::minutes(3),
            Some(now - chrono::Duration::minutes(1)),
        );
        let deleted_2 = build_image(
            Uuid::new_v4(),
            user_id,
            "deleted-2.jpg",
            "hash-2",
            now - chrono::Duration::minutes(4),
            Some(now - chrono::Duration::minutes(2)),
        );
        let deleted_3 = build_image(
            Uuid::new_v4(),
            user_id,
            "deleted-3.jpg",
            "hash-3",
            now - chrono::Duration::minutes(5),
            Some(now - chrono::Duration::minutes(3)),
        );

        service
            .image_repository
            .create_image(&deleted_1)
            .await
            .unwrap();
        service
            .image_repository
            .create_image(&deleted_2)
            .await
            .unwrap();
        service
            .image_repository
            .create_image(&deleted_3)
            .await
            .unwrap();

        let page1 = service
            .get_deleted_images_paginated(user_id, 1, 2)
            .await
            .unwrap();
        assert_eq!(page1.total, 3);
        assert_eq!(page1.data.len(), 2);
        assert!(page1.has_next);
        assert!(page1.data.iter().all(|img| img.total_count.is_none()));

        let page2 = service
            .get_deleted_images_paginated(user_id, 2, 2)
            .await
            .unwrap();
        assert_eq!(page2.total, 3);
        assert_eq!(page2.data.len(), 1);
        assert!(!page2.has_next);
        assert!(page2.data.iter().all(|img| img.total_count.is_none()));
    }

    #[tokio::test]
    async fn test_get_deleted_images_paginated_rejects_invalid_params() {
        let service = setup_service().await;
        let user_id = Uuid::new_v4();

        let result = service.get_deleted_images_paginated(user_id, 0, 20).await;
        assert!(matches!(result, Err(AppError::InvalidPagination)));

        let result = service.get_deleted_images_paginated(user_id, 1, 0).await;
        assert!(matches!(result, Err(AppError::InvalidPagination)));
    }

    /// 测试 ImageInfo 序列化
    #[test]
    fn test_image_info_serialization() {
        let result = ImageInfo {
            id: Uuid::new_v4(),
            filename: "test.jpg".to_string(),
            user_id: Uuid::new_v4(),
        };

        let json = serde_json::to_string(&result).expect("Failed to serialize");
        let parsed: ImageInfo = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(parsed.id, result.id);
        assert_eq!(parsed.filename, result.filename);
        assert_eq!(parsed.user_id, result.user_id);
    }
}
