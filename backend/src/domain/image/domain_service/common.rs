use super::*;

impl<I: ImageRepository, C: CategoryRepository> ImageDomainService<I, C> {
    pub(super) fn validate_image_key(image_key: &str) -> Result<(), AppError> {
        if image_key.len() == 64 && image_key.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(());
        }
        Err(AppError::ValidationError(
            "图片键无效，必须是 64 位十六进制哈希".to_string(),
        ))
    }

    pub(super) fn normalize_tags(tags: &[String]) -> Result<Vec<String>, AppError> {
        let mut seen = HashSet::new();
        let mut normalized = Vec::new();

        for raw_tag in tags {
            let tag = raw_tag.trim();
            if tag.is_empty() {
                continue;
            }
            if tag.chars().count() > MAX_TAG_LENGTH || tag.chars().any(char::is_control) {
                return Err(AppError::ValidationError("标签格式无效".to_string()));
            }

            let lowered = tag.to_lowercase();
            if seen.insert(lowered.clone()) {
                normalized.push(lowered);
            }
        }

        if normalized.len() > MAX_TAGS_PER_IMAGE {
            return Err(AppError::ValidationError(format!(
                "标签数量不能超过 {}",
                MAX_TAGS_PER_IMAGE
            )));
        }

        Ok(normalized)
    }

    pub fn new(
        deps: ImageDomainServiceDependencies,
        image_repository: I,
        category_repository: C,
    ) -> Self {
        Self {
            pool: deps.pool,
            redis: deps.redis,
            config: deps.config,
            image_repository,
            category_repository,
            image_processor: deps.image_processor,
            file_save_queue: deps.file_save_queue,
            storage_manager: deps.storage_manager,
        }
    }

    pub(super) async fn invalidate_hash_cache_for_user(
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

    pub(super) async fn invalidate_hash_cache_patterns_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<(), AppError> {
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

    #[tracing::instrument(skip(self, hash), fields(hash = %hash))]
    pub async fn check_image_hash(
        &self,
        hash: &str,
        strategy: &str,
        user_id: Uuid,
    ) -> Result<Option<ImageInfo>, AppError> {
        let cache_info_key = HashCache::existing_info(hash, strategy, user_id);

        if let Some(manager) = self.redis.as_ref() {
            let mut redis = manager.clone();
            if let Ok(Some(cached)) = Cache::get::<ImageInfo, _>(&mut redis, &cache_info_key).await
            {
                info!("Hash cache hit for image hash: {}", hash);
                return Ok(Some(cached));
            }
        }

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

        if let (Some(info), Some(manager)) = (&existing, self.redis.as_ref()) {
            let mut redis = manager.clone();
            let cache_ttl = self.config.cache.list_ttl;
            let _ = Cache::set(&mut redis, &cache_info_key, info, cache_ttl).await;
            let hash_cache_key = HashCache::image_hash(hash, strategy);
            let _ = Cache::set(&mut redis, &hash_cache_key, "1", 3600).await;
        }

        Ok(existing)
    }
}
