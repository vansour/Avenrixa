use crate::audit::log_audit;
use crate::cache::{Cache, ImageCache, HashCache};
use crate::db::AppState;
use crate::error::AppError;
use crate::file_queue::FileSaveResult;
use crate::middleware::AuthUser;
use crate::models::*;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use axum_extra::extract::Multipart;
use chrono::Utc;
use redis::AsyncCommands;
use tracing::{error, info, warn};
use uuid::Uuid;
use crate::utils::write_file_with_retry;

/// 检查图片 hash 是否已存在（使用 Redis 缓存层）
async fn check_image_hash_with_cache(
    state: &crate::db::AppState,
    hash: &str,
    strategy: &str,
    user_id: uuid::Uuid,
) -> Result<Option<ImageInfo>, AppError>
{
    // 先检查 Redis 缓存
    let _cache_key = HashCache::image_hash(hash, strategy);
    let cache_info_key = HashCache::existing_info(hash, strategy, user_id);
    let mut redis = state.redis.clone();

    // 尝试从缓存获取现有图片信息
    if let Ok(Some(cached_info)) = Cache::get::<ImageInfo, _>(&mut redis, &cache_info_key).await {
        // 缓存命中，返回缓存的图片信息
        info!("Hash cache hit for image hash: {}", hash);
        return Ok(Some(cached_info));
    }

    // 缓存未命中，查询数据库
    info!("Hash cache miss for image hash: {}, querying database", hash);
    let existing_row = match strategy {
        "global" => {
            // 全局去重：检查所有用户
            sqlx::query_as::<_, ImageInfo>(
                "SELECT id, filename, user_id FROM images WHERE hash = $1 AND deleted_at IS NULL LIMIT 1"
            )
            .bind(hash)
            .fetch_optional(&state.pool)
            .await?
        }
        _ => {
            // 用户内去重（默认）
            sqlx::query_as::<_, ImageInfo>(
                "SELECT id, filename, user_id FROM images WHERE hash = $1 AND user_id = $2 AND deleted_at IS NULL"
            )
            .bind(hash)
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await?
        }
    };

    // 将查询结果存入缓存
    if let Some(ref info) = existing_row {
        // 设置缓存 TTL，使用 list_ttl (5分钟) 用于图片列表缓存
        let cache_ttl = state.config.cache.list_ttl;
        let _ = Cache::set(&mut redis, &cache_info_key, info, cache_ttl).await;

        // 将 hash 添加到已存在集合（用于快速检查）
        let hash_cache_key = HashCache::image_hash(hash, strategy);
        let _ = Cache::set(&mut redis, &hash_cache_key, "1", 3600).await;
    }

    Ok(existing_row)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
struct ImageInfo {
    id: Uuid,
    filename: String,
    user_id: Uuid,
}

pub async fn upload_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<Image>, AppError> {
    while let Some(field) = multipart.next_field().await.map_err(|_| {
        error!("Failed to read multipart field");
        AppError::InvalidImageFormat
    })? {
        let name = field.name().unwrap_or("");
        if name == "file" {
            let filename = field.file_name().unwrap_or("unknown").to_string();
            let content_type = field.content_type().map(|ct| ct.to_string());
            let data = field.bytes().await.map_err(|_| {
                error!("Failed to read file bytes");
                AppError::InvalidImageFormat
            })?.to_vec();

            if data.is_empty() {
                warn!("Empty file uploaded");
                return Err(AppError::InvalidImageFormat);
            }

            // 验证文件魔数
            crate::image_processor::ImageProcessor::validate_image_bytes(&data)?;

            if !content_type.as_deref().is_some_and(|ct| crate::image_processor::ImageProcessor::is_image(Some(ct))) {
                warn!("Invalid file type: {:?}", content_type);
                return Err(AppError::InvalidImageFormat);
            }

            let ext = crate::image_processor::ImageProcessor::get_extension(&filename);
            if !state.config.storage.allowed_extensions.contains(&ext) {
                warn!("Unsupported extension: {}", ext);
                return Err(AppError::InvalidImageFormat);
            }

            // 使用 spawn_blocking 处理图片，避免阻塞事件循环
            let processor = state.image_processor.clone();
            let data_for_processing = data;
            let (compressed, thumbnail) = tokio::task::spawn_blocking(move || {
                processor.process(&data_for_processing)
            }).await.map_err(|e| {
                error!("Failed to join blocking task: {}", e);
                AppError::Internal(e.into())
            })??;

            let hash = crate::image_processor::ImageProcessor::calculate_hash(&compressed);
            let compressed_size = compressed.len() as i64;

            // 使用 Redis 缓存层检查图片 hash 是否已存在
            let dedup_strategy = &state.config.image.dedup_strategy;
            let existing_info = check_image_hash_with_cache(&state, &hash, dedup_strategy, auth_user.id).await?;

            if let Some(info) = existing_info {
                // 用户内去重：直接返回现有图片
                // 全局去重：仅当图片属于当前用户时才返回
                if dedup_strategy == "user" || info.user_id == auth_user.id {
                    info!("Duplicate image detected (cache hit), returning existing: {} (strategy: {})", info.id, dedup_strategy);
                    return Ok(Json(Image {
                        id: info.id,
                        user_id: auth_user.id,
                        category_id: None,
                        filename: info.filename,
                        thumbnail: Some(format!("{}.jpg", info.id)),
                        original_filename: Some(filename.clone()),
                        size: compressed_size,
                        hash,
                        format: ext,
                        views: 0,
                        status: "active".to_string(),
                        expires_at: None,
                        deleted_at: None,
                        created_at: Utc::now(),
                        total_count: None,
                    }));
                } else {
                    info!("Duplicate hash detected but belongs to different user, allowing new upload");
                }
            }

            let image_id = Uuid::new_v4();
            let stored_filename = format!("{}.{}", image_id, ext);
            let thumbnail_filename = format!("{}.jpg", image_id);

            // 使用任务队列保存文件，确保文件写入完成后再返回
            let storage_path = format!("{}/{}", state.config.storage.path, stored_filename);
            let thumbnail_path = format!("{}/{}", state.config.storage.thumbnail_path, thumbnail_filename);

            // 提交文件保存任务到队列
            let save_task = crate::file_queue::FileSaveTask {
                image_id: image_id.to_string(),
                storage_path: storage_path.clone(),
                thumbnail_path: thumbnail_path.clone(),
                image_data: compressed.clone(),
                thumbnail_data: thumbnail.clone(),
            };

            // 等待文件保存完成
            match state.file_save_queue.submit(save_task).await {
                Ok(()) => {
                    info!("Files saved successfully for image: {}", image_id);
                }
                Err(FileSaveResult::ImageFailed { image_id: id, error }) => {
                    error!("Failed to save image file [{}]: {}", id, error);
                    return Err(AppError::IoError(std::io::Error::other(
                        format!("文件保存失败: {}", error),
                    )));
                }
                Err(FileSaveResult::ThumbnailFailed { image_id: id, error }) => {
                    error!("Failed to save thumbnail [{}]: {}", id, error);
                    // 缩略图失败不影响主要功能，继续返回
                    warn!("Thumbnail save failed, but continuing with image record");
                }
                Err(e) => {
                    error!("File save queue error: {:?}", e);
                    return Err(AppError::Internal(anyhow::anyhow!("文件保存队列错误: {:?}", e)));
                }
            }

            let image = Image {
                id: image_id,
                user_id: auth_user.id,
                category_id: None,
                filename: stored_filename,
                thumbnail: Some(thumbnail_filename),
                original_filename: Some(filename.clone()),
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

            sqlx::query(
                "INSERT INTO images (id, user_id, category_id, filename, thumbnail, original_filename, size, hash, format, views, status, expires_at, deleted_at, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)"
            )
            .bind(image.id)
            .bind(image.user_id)
            .bind(image.category_id)
            .bind(&image.filename)
            .bind(&image.thumbnail)
            .bind(&image.original_filename)
            .bind(image.size)
            .bind(&image.hash)
            .bind(&image.format)
            .bind(image.views)
            .bind(&image.status)
            .bind(image.expires_at)
            .bind(image.deleted_at)
            .bind(image.created_at)
            .execute(&state.pool)
            .await?;

            let cache_key = format!("{}{}", state.config.redis.key_prefix, image_id);
            let mut redis = state.redis.clone();
            let _: Result<(), _> = redis.set_ex(cache_key, &storage_path, state.config.redis.ttl).await;

            info!("Image uploaded: {} by {}", image_id, auth_user.username);
            log_audit(&state.pool, Some(auth_user.id), "image.upload", "image", Some(image_id), None, None).await;
            return Ok(Json(image));
        }
    }

    warn!("No file field found in multipart");
    Err(AppError::InvalidImageFormat)
}

/// 允许的排序字段白名单（防止 SQL 注入）
const VALID_SORT_FIELDS: &[&str] = &[
    "created_at", "size", "views", "filename", "hash"
];

/// 允许的排序方向白名单
const VALID_SORT_ORDERS: &[&str] = &["ASC", "DESC"];

pub async fn get_images(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Paginated<Image>>, AppError> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    // 使用白名单验证排序字段，防止 SQL 注入
    let sort_by = VALID_SORT_FIELDS
        .iter()
        .find(|&&f| f == params.sort_by.as_deref().unwrap_or("created_at"))
        .copied()
        .unwrap_or("created_at");

    // 使用白名单验证排序方向
    let sort_order = VALID_SORT_ORDERS
        .iter()
        .find(|&&o| o == params.sort_order.as_deref().unwrap_or("DESC"))
        .copied()
        .unwrap_or("DESC");

    let search = params.search.as_deref();
    let category_id = params.category_id;
    let tag = params.tag.as_deref();

    // 尝试从缓存获取
    let cache_key = ImageCache::list(auth_user.id, page, page_size, category_id, sort_by, sort_order);
    if search.is_none() && tag.is_none() {
        let mut redis = state.redis.clone();
        if let Ok(Some(cached)) = Cache::get::<Paginated<Image>, _>(&mut redis, &cache_key).await {
            return Ok(Json(cached));
        }
    }

    // 使用已验证的 sort_by 和 sort_order，避免 format! 拼接
    // order_clause 值已直接嵌入到静态 SQL 语句中
    let _order_clause = match (sort_by, sort_order) {
        ("created_at", "ASC") => "ORDER BY created_at ASC",
        ("created_at", "DESC") => "ORDER BY created_at DESC",
        ("size", "ASC") => "ORDER BY size ASC",
        ("size", "DESC") => "ORDER BY size DESC",
        ("views", "ASC") => "ORDER BY views ASC",
        ("views", "DESC") => "ORDER BY views DESC",
        ("filename", "ASC") => "ORDER BY filename ASC",
        ("filename", "DESC") => "ORDER BY filename DESC",
        ("hash", "ASC") => "ORDER BY hash ASC",
        ("hash", "DESC") => "ORDER BY hash DESC",
        _ => "ORDER BY created_at DESC", // 默认值
    };

    // 根据参数选择合适的查询，使用完全静态的 SQL 语句
    let (images, total) = match (search, category_id, tag) {
        // 搜索 + 分类 + 标签
        (Some(s), Some(cid), Some(t)) if !s.is_empty() => {
            let rows = sqlx::query_as::<_, Image>(
                "SELECT *, COUNT(*) OVER() as total_count FROM images
                 WHERE user_id = $1 AND deleted_at IS NULL AND status = 'active'
                 AND category_id = $2 AND $3 IN (SELECT tag FROM image_tags WHERE image_id = images.id)
                 AND (filename ILIKE $4 OR id::text IN (SELECT tag FROM image_tags WHERE tag ILIKE $5 AND image_id = images.id))
                 ORDER BY created_at DESC LIMIT $6 OFFSET $7"
            )
            .bind(auth_user.id)
            .bind(cid)
            .bind(t)
            .bind(format!("%{}%", s))
            .bind(format!("%{}%", s))
            .bind(page_size)
            .bind(offset)
            .fetch_all(&state.pool)
            .await?;
            let total = rows.first().and_then(|img| img.total_count).unwrap_or(0) as i64;
            (rows, total)
        }
        // 搜索 + 分类
        (Some(s), Some(cid), None) if !s.is_empty() => {
            let rows = sqlx::query_as::<_, Image>(
                "SELECT *, COUNT(*) OVER() as total_count FROM images
                 WHERE user_id = $1 AND deleted_at IS NULL AND status = 'active'
                 AND category_id = $2
                 AND (filename ILIKE $3 OR id::text IN (SELECT tag FROM image_tags WHERE tag ILIKE $4 AND image_id = images.id))
                 ORDER BY created_at DESC LIMIT $5 OFFSET $6"
            )
            .bind(auth_user.id)
            .bind(cid)
            .bind(format!("%{}%", s))
            .bind(format!("%{}%", s))
            .bind(page_size)
            .bind(offset)
            .fetch_all(&state.pool)
            .await?;
            let total = rows.first().and_then(|img| img.total_count).unwrap_or(0) as i64;
            (rows, total)
        }
        // 搜索 + 标签
        (Some(s), None, Some(t)) if !s.is_empty() => {
            let rows = sqlx::query_as::<_, Image>(
                "SELECT *, COUNT(*) OVER() as total_count FROM images
                 WHERE user_id = $1 AND deleted_at IS NULL AND status = 'active'
                 AND $2 IN (SELECT tag FROM image_tags WHERE image_id = images.id)
                 AND (filename ILIKE $3 OR id::text IN (SELECT tag FROM image_tags WHERE tag ILIKE $4 AND image_id = images.id))
                 ORDER BY created_at DESC LIMIT $5 OFFSET $6"
            )
            .bind(auth_user.id)
            .bind(t)
            .bind(format!("%{}%", s))
            .bind(format!("%{}%", s))
            .bind(page_size)
            .bind(offset)
            .fetch_all(&state.pool)
            .await?;
            let total = rows.first().and_then(|img| img.total_count).unwrap_or(0) as i64;
            (rows, total)
        }
        // 搜索
        (Some(s), None, None) if !s.is_empty() => {
            let rows = sqlx::query_as::<_, Image>(
                "SELECT *, COUNT(*) OVER() as total_count FROM images
                 WHERE user_id = $1 AND deleted_at IS NULL AND status = 'active'
                 AND (filename ILIKE $2 OR id::text IN (SELECT tag FROM image_tags WHERE tag ILIKE $3 AND image_id = images.id))
                 ORDER BY created_at DESC LIMIT $4 OFFSET $5"
            )
            .bind(auth_user.id)
            .bind(format!("%{}%", s))
            .bind(format!("%{}%", s))
            .bind(page_size)
            .bind(offset)
            .fetch_all(&state.pool)
            .await?;
            let total = rows.first().and_then(|img| img.total_count).unwrap_or(0) as i64;
            (rows, total)
        }
        // 分类 + 标签
        (None, Some(cid), Some(t)) => {
            let rows = sqlx::query_as::<_, Image>(
                "SELECT *, COUNT(*) OVER() as total_count FROM images
                 WHERE user_id = $1 AND deleted_at IS NULL AND status = 'active'
                 AND category_id = $2 AND $3 IN (SELECT tag FROM image_tags WHERE image_id = images.id)
                 ORDER BY created_at DESC LIMIT $4 OFFSET $5"
            )
            .bind(auth_user.id)
            .bind(cid)
            .bind(t)
            .bind(page_size)
            .bind(offset)
            .fetch_all(&state.pool)
            .await?;
            let total = rows.first().and_then(|img| img.total_count).unwrap_or(0) as i64;
            (rows, total)
        }
        // 分类
        (None, Some(cid), None) => {
            let rows = sqlx::query_as::<_, Image>(
                "SELECT *, COUNT(*) OVER() as total_count FROM images
                 WHERE user_id = $1 AND deleted_at IS NULL AND status = 'active' AND category_id = $2
                 ORDER BY created_at DESC LIMIT $3 OFFSET $4"
            )
            .bind(auth_user.id)
            .bind(cid)
            .bind(page_size)
            .bind(offset)
            .fetch_all(&state.pool)
            .await?;
            let total = rows.first().and_then(|img| img.total_count).unwrap_or(0) as i64;
            (rows, total)
        }
        // 标签
        (None, None, Some(t)) => {
            let rows = sqlx::query_as::<_, Image>(
                "SELECT *, COUNT(*) OVER() as total_count FROM images
                 WHERE user_id = $1 AND deleted_at IS NULL AND status = 'active'
                 AND $2 IN (SELECT tag FROM image_tags WHERE image_id = images.id)
                 ORDER BY created_at DESC LIMIT $3 OFFSET $4"
            )
            .bind(auth_user.id)
            .bind(t)
            .bind(page_size)
            .bind(offset)
            .fetch_all(&state.pool)
            .await?;
            let total = rows.first().and_then(|img| img.total_count).unwrap_or(0) as i64;
            (rows, total)
        }
        // 无筛选条件
        (None, None, None) => {
            let rows = sqlx::query_as::<_, Image>(
                "SELECT *, COUNT(*) OVER() as total_count FROM images
                 WHERE user_id = $1 AND deleted_at IS NULL AND status = 'active'
                 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
            )
            .bind(auth_user.id)
            .bind(page_size)
            .bind(offset)
            .fetch_all(&state.pool)
            .await?;
            let total = rows.first().and_then(|img| img.total_count).unwrap_or(0) as i64;
            (rows, total)
        }
        // 空搜索
        _ => {
            let rows = sqlx::query_as::<_, Image>(
                "SELECT *, COUNT(*) OVER() as total_count FROM images
                 WHERE user_id = $1 AND deleted_at IS NULL AND status = 'active'
                 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
            )
            .bind(auth_user.id)
            .bind(page_size)
            .bind(offset)
            .fetch_all(&state.pool)
            .await?;
            let total = rows.first().and_then(|img| img.total_count).unwrap_or(0) as i64;
            (rows, total)
        }
    };

    // 清理 total_count 字段（避免序列化）
    let mut images = images;
    for img in &mut images {
        img.total_count = None;
    }

    // 根据配置决定是否进行文件存在检查
    let valid_images = if state.config.storage.enable_file_check {
        let storage_path_base = &state.config.storage.path;
        let thumbnail_path_base = &state.config.storage.thumbnail_path;
        let concurrent_threshold = state.config.storage.file_check_concurrent_threshold;

        if images.len() > concurrent_threshold {
            // 图片数量较多时，使用并发检查
            use futures::future::join_all;
            let existence_checks: Vec<_> = images.iter().map(|img| async {
                let storage_path = format!("{}/{}", storage_path_base, img.filename);
                let thumbnail_path = format!("{}/{}.jpg", thumbnail_path_base, img.id);
                tokio::join!(
                    tokio::fs::try_exists(storage_path),
                    tokio::fs::try_exists(thumbnail_path)
                )
            }).collect();

            let results = join_all(existence_checks).await;
            images.into_iter()
                .zip(results)
                .filter(|(_img, (storage_exists, thumb_exists))| {
                    *storage_exists.as_ref().unwrap_or(&false) && *thumb_exists.as_ref().unwrap_or(&false)
                })
                .map(|(img, _)| img)
                .collect()
        } else {
            // 图片数量较少时，使用串行检查
            let mut valid_images = Vec::new();
            for img in images {
                let storage_path = format!("{}/{}", storage_path_base, img.filename);
                let thumbnail_path = format!("{}/{}.jpg", thumbnail_path_base, img.id);
                let storage_exists = tokio::fs::try_exists(&storage_path).await.unwrap_or(false);
                let thumbnail_exists = tokio::fs::try_exists(&thumbnail_path).await.unwrap_or(false);
                if storage_exists && thumbnail_exists {
                    valid_images.push(img);
                }
            }
            valid_images
        }
    } else {
        // 文件检查已禁用，直接返回数据库结果
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

    // 缓存结果（仅对简单查询）
    if search.is_none() && tag.is_none() {
        let mut redis = state.redis.clone();
        let ttl = state.config.cache.list_ttl;
        let _ = Cache::set(&mut redis, &cache_key, &result, ttl).await;
    }

    Ok(Json(result))
}

pub async fn get_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Image>, AppError> {
    let image: Image = sqlx::query_as(
        "SELECT * FROM images WHERE id = $1 AND user_id = $2"
    )
    .bind(id)
    .bind(auth_user.id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::ImageNotFound)?;

    sqlx::query("UPDATE images SET views = views + 1 WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    Ok(Json(image))
}

pub async fn update_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCategoryRequest>,
) -> Result<(), AppError> {
    if let Some(tags) = &req.tags {
        // 先删除旧标签
        sqlx::query("DELETE FROM image_tags WHERE image_id = $1")
            .bind(id)
            .execute(&state.pool)
            .await?;

        // 插入新标签
        for tag in tags {
            if !tag.is_empty() {
                sqlx::query("INSERT INTO image_tags (image_id, tag) VALUES ($1, $2)")
                    .bind(id)
                    .bind(tag)
                    .execute(&state.pool)
                    .await?;
            }
        }
    }

    // 更新分类
    if let Some(cat_id) = req.category_id {
        sqlx::query("UPDATE images SET category_id = $1 WHERE id = $2 AND user_id = $3")
            .bind(cat_id)
            .bind(id)
            .bind(auth_user.id)
            .execute(&state.pool)
            .await?;
    }

    // 清除缓存
    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(())
}

pub async fn rename_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RenameRequest>,
) -> Result<(), AppError> {
    if req.filename.is_empty() {
        return Err(AppError::InvalidPagination);
    }

    sqlx::query("UPDATE images SET original_filename = $1 WHERE id = $2 AND user_id = $3")
        .bind(&req.filename)
        .bind(id)
        .bind(auth_user.id)
        .execute(&state.pool)
        .await?;

    // 清除缓存
    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(())
}

pub async fn set_expiry(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SetExpiryRequest>,
) -> Result<(), AppError> {
    sqlx::query("UPDATE images SET expires_at = $1 WHERE id = $2 AND user_id = $3")
        .bind(req.expires_at)
        .bind(id)
        .bind(auth_user.id)
        .execute(&state.pool)
        .await?;

    // 清除缓存
    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(())
}

pub async fn delete_images(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(req): Json<DeleteRequest>,
) -> Result<(), AppError> {
    if req.permanent {
        // 永久删除
        for id in &req.image_ids {
            let img_result: Option<Image> = sqlx::query_as(
                "SELECT filename FROM images WHERE id = $1 AND user_id = $2"
            )
            .bind(id)
            .bind(auth_user.id)
            .fetch_optional(&state.pool)
            .await?;

            if let Some(img) = img_result {
                let storage_path = format!("{}/{}", state.config.storage.path, img.filename);
                let thumbnail_path = format!("{}/{}.jpg", state.config.storage.thumbnail_path, id);
                let _ = tokio::fs::remove_file(&storage_path).await;
                let _ = tokio::fs::remove_file(&thumbnail_path).await;
            }

            sqlx::query("DELETE FROM images WHERE id = $1 AND user_id = $2")
                .bind(id)
                .bind(auth_user.id)
                .execute(&state.pool)
                .await?;
        }
    } else {
        // 软删除
        sqlx::query("UPDATE images SET deleted_at = $1 WHERE id = ANY($2) AND user_id = $3")
            .bind(Utc::now())
            .bind(&req.image_ids)
            .bind(auth_user.id)
            .execute(&state.pool)
            .await?;
    }

    // 清除缓存
    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(())
}

pub async fn get_deleted_images(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<Vec<Image>>, AppError> {
    let images: Vec<Image> = sqlx::query_as(
        "SELECT * FROM images WHERE user_id = $1 AND deleted_at IS NOT NULL ORDER BY deleted_at DESC"
    )
    .bind(auth_user.id)
    .fetch_all(&state.pool)
    .await?;

    // 根据配置决定是否进行文件存在检查
    let valid_images = if state.config.storage.enable_file_check {
        let storage_path_base = &state.config.storage.path;
        let thumbnail_path_base = &state.config.storage.thumbnail_path;
        let concurrent_threshold = state.config.storage.file_check_concurrent_threshold;

        if images.len() > concurrent_threshold {
            // 图片数量较多时，使用并发检查
            use futures::future::join_all;
            let existence_checks: Vec<_> = images.iter().map(|img| async {
                let storage_path = format!("{}/{}", storage_path_base, img.filename);
                let thumbnail_path = format!("{}/{}.jpg", thumbnail_path_base, img.id);
                tokio::join!(
                    tokio::fs::try_exists(storage_path),
                    tokio::fs::try_exists(thumbnail_path)
                )
            }).collect();

            let results = join_all(existence_checks).await;
            images.into_iter()
                .zip(results)
                .filter(|(_img, (storage_exists, thumb_exists))| {
                    *storage_exists.as_ref().unwrap_or(&false) && *thumb_exists.as_ref().unwrap_or(&false)
                })
                .map(|(img, _)| img)
                .collect()
        } else {
            // 图片数量较少时，使用串行检查
            let mut valid_images = Vec::new();
            for img in images {
                let storage_path = format!("{}/{}", storage_path_base, img.filename);
                let thumbnail_path = format!("{}/{}.jpg", thumbnail_path_base, img.id);
                let storage_exists = tokio::fs::try_exists(&storage_path).await.unwrap_or(false);
                let thumbnail_exists = tokio::fs::try_exists(&thumbnail_path).await.unwrap_or(false);
                if storage_exists && thumbnail_exists {
                    valid_images.push(img);
                }
            }
            valid_images
        }
    } else {
        // 文件检查已禁用，直接返回数据库结果
        images
    };

    Ok(Json(valid_images))
}

pub async fn restore_images(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(req): Json<RestoreRequest>,
) -> Result<(), AppError> {
    sqlx::query("UPDATE images SET deleted_at = NULL WHERE id = ANY($1) AND user_id = $2")
        .bind(&req.image_ids)
        .bind(auth_user.id)
        .execute(&state.pool)
        .await?;

    // 清除缓存
    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(())
}

pub async fn duplicate_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(_id): Path<Uuid>,
    Json(req): Json<DuplicateRequest>,
) -> Result<Json<Image>, AppError> {
    // 获取原图
    let original: Image = sqlx::query_as(
        "SELECT * FROM images WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL"
    )
    .bind(req.image_id)
    .bind(auth_user.id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::ImageNotFound)?;

    // 检查源图片是否属于当前用户
    if original.user_id != auth_user.id {
        return Err(AppError::Forbidden);
    }

    let new_id = Uuid::new_v4();
    let new_filename = format!("copy_{}", original.filename);

    // 复制图片文件
    let src_path = format!("{}/{}", state.config.storage.path, original.filename);
    let dst_path = format!("{}/{}", state.config.storage.path, new_filename);
    tokio::fs::copy(&src_path, &dst_path).await?;

    // 复制缩略图
    if let Some(ref thumb) = original.thumbnail {
        let src_thumb = format!("{}/{}", state.config.storage.thumbnail_path, thumb);
        let dst_thumb = format!("{}/{}.jpg", state.config.storage.thumbnail_path, new_id);
        let _ = tokio::fs::copy(&src_thumb, &dst_thumb).await;
    }

    let duplicated = Image {
        id: new_id,
        user_id: auth_user.id,
        category_id: original.category_id,
        filename: new_filename,
        thumbnail: Some(format!("{}.jpg", new_id)),
        original_filename: Some(format!("copy_of_{}", original.original_filename.unwrap_or(original.filename))),
        size: original.size,
        hash: format!("{}-{}", original.hash, new_id), // 添加后缀避免 hash 冲突
        format: original.format,
        views: 0,
        status: "active".to_string(),
        expires_at: original.expires_at,
        deleted_at: None,
        created_at: Utc::now(),
        total_count: None,
    };

    sqlx::query(
        "INSERT INTO images (id, user_id, category_id, filename, thumbnail, original_filename, size, hash, format, views, status, expires_at, deleted_at, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)"
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
    .execute(&state.pool)
    .await?;

    // 清除用户缓存
    let mut redis = state.redis.clone();
    let _ = Cache::del_pattern(&mut redis, &format!("images:list:{}:*", auth_user.id)).await;

    info!("Image duplicated: {} -> {} by {}", req.image_id, new_id, auth_user.username);
    Ok(Json(duplicated))
}

pub async fn edit_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<EditImageRequest>,
) -> Result<Json<EditImageResponse>, AppError> {
    let image: Image = sqlx::query_as(
        "SELECT * FROM images WHERE id = $1 AND user_id = $2"
    )
    .bind(id)
    .bind(auth_user.id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::ImageNotFound)?;

    let storage_path = format!("{}/{}", state.config.storage.path, image.filename);

    let original_data = tokio::fs::read(&storage_path).await?;

    let crop = req.crop.map(|c| (c.x as u32, c.y as u32, c.width as u32, c.height as u32));
    let filters = req.filters.as_ref().map(|f| crate::image_processor::FilterParams {
        brightness: f.brightness,
        contrast: f.contrast,
        saturation: f.saturation,
        grayscale: f.grayscale,
        sepia: f.sepia,
    });
    let watermark = req.watermark.as_ref().map(|w| crate::image_processor::WatermarkParams {
        text: w.text.clone(),
        position: w.position.clone(),
        opacity: w.opacity,
    });

    let edited_data = state.image_processor.edit_image(
        &original_data,
        crop,
        req.rotate,
        &filters,
        &watermark,
        req.convert_format.as_deref(),
    )
    .map_err(|e| AppError::ImageProcessingFailed { source: e })?;

    // 异步保存编辑后的图片（带重试）
    let max_retries = 3;
    let storage_path_edit = storage_path.clone();
    let edited_data_for_save = edited_data.clone();
    tokio::spawn(async move {
        if let Err(e) = write_file_with_retry(&storage_path_edit, &edited_data_for_save, max_retries).await {
            error!("Failed to write edited image after {} retries: {}", max_retries, e);
        }
    });

    // 重新生成缩略图
    let img = image::load_from_memory(&edited_data)
        .map_err(|e| AppError::ImageProcessingFailed { source: e.into() })?;

    let thumbnail = state.image_processor.generate_thumbnail(&img)
        .map_err(|e| AppError::ImageProcessingFailed { source: e })?;

    let thumbnail_path = format!("{}/{}.jpg", state.config.storage.thumbnail_path, id);
    let max_retries = 3;
    let thumbnail_path_clone_thumbs = thumbnail_path.clone();
    tokio::spawn(async move {
        if let Err(e) = write_file_with_retry(&thumbnail_path_clone_thumbs, &thumbnail, max_retries).await {
            error!("Failed to write thumbnail after {} retries: {}", max_retries, e);
        }
    });

    // 清除缓存
    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(Json(EditImageResponse {
        id,
        edited_url: format!("/images/{}", id),
        thumbnail_url: format!("/thumbnails/{}.jpg", id),
    }))
}
