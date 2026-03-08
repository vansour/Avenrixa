use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::models::*;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use axum_extra::extract::Multipart;
use tokio::io::AsyncWriteExt;
use tracing::{error, warn};
use uuid::Uuid;

#[tracing::instrument(skip(state, auth_user, multipart), fields(user_id = %auth_user.id))]
pub async fn upload_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<Image>, AppError> {
    let service = state
        .image_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Image service not found"
        )))?;

    while let Some(field) = multipart.next_field().await.map_err(|_| {
        error!("Failed to read multipart field");
        AppError::InvalidImageFormat
    })? {
        let name = field.name().unwrap_or("");
        if name == "file" {
            let filename = field.file_name().unwrap_or("unknown").to_string();
            let content_type = field.content_type().map(|ct| ct.to_string());

            // 创建临时文件进行流式写入
            let temp_dir = std::env::temp_dir();
            let temp_file_path = temp_dir.join(format!("upload_{}.tmp", Uuid::new_v4()));
            let mut file = tokio::fs::File::create(&temp_file_path)
                .await
                .map_err(|e| {
                    error!("Failed to create temp file: {}", e);
                    AppError::Internal(e.into())
                })?;

            let mut field_stream = field;
            let mut size: usize = 0;
            while let Some(chunk) = field_stream.chunk().await.map_err(|_| {
                error!("Failed to read field chunk");
                AppError::InvalidImageFormat
            })? {
                file.write_all(&chunk).await.map_err(|e| {
                    error!("Failed to write to temp file: {}", e);
                    AppError::Internal(e.into())
                })?;
                size += chunk.len();
                if size > state.config.server.max_upload_size {
                    let _ = tokio::fs::remove_file(&temp_file_path).await;
                    return Err(AppError::InvalidImageFormat);
                }
            }

            if size == 0 {
                warn!("Empty file uploaded");
                let _ = tokio::fs::remove_file(&temp_file_path).await;
                return Err(AppError::InvalidImageFormat);
            }

            let image = service
                .upload_image_from_file(
                    auth_user.id,
                    &auth_user.username,
                    filename,
                    temp_file_path,
                    content_type,
                )
                .await?;

            return Ok(Json(image));
        }
    }

    warn!("No file field found in multipart");
    Err(AppError::InvalidImageFormat)
}

/// 允许的排序字段白名单（防止 SQL 注入）
const VALID_SORT_FIELDS: &[&str] = &["created_at", "size", "views", "filename", "hash"];

/// 允许的排序方向白名单
const VALID_SORT_ORDERS: &[&str] = &["ASC", "DESC"];

pub async fn get_images(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Paginated<Image>>, AppError> {
    let service = state
        .image_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Image service not found"
        )))?;

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    let sort_by = VALID_SORT_FIELDS
        .iter()
        .find(|&&f| f == params.sort_by.as_deref().unwrap_or("created_at"))
        .copied()
        .unwrap_or("created_at");

    let sort_order = VALID_SORT_ORDERS
        .iter()
        .find(|&&o| o == params.sort_order.as_deref().unwrap_or("DESC"))
        .copied()
        .unwrap_or("DESC");

    let result = service
        .get_images(
            auth_user.id,
            page,
            page_size,
            sort_by,
            sort_order,
            params.search.as_deref(),
            params.category_id,
            params.tag.as_deref(),
        )
        .await?;

    Ok(Json(result))
}

pub async fn get_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Image>, AppError> {
    let service = state
        .image_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Image service not found"
        )))?;
    let image = service.increment_views(id, auth_user.id).await?;
    Ok(Json(image))
}

pub async fn update_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCategoryRequest>,
) -> Result<(), AppError> {
    let service = state
        .image_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Image service not found"
        )))?;

    service
        .update_image_category(id, auth_user.id, req.category_id, req.tags.as_deref())
        .await?;

    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(())
}

pub async fn rename_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RenameRequest>,
) -> Result<(), AppError> {
    let service = state
        .image_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Image service not found"
        )))?;

    service
        .rename_image(id, auth_user.id, &req.filename)
        .await?;
    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(())
}

pub async fn set_expiry(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SetExpiryRequest>,
) -> Result<(), AppError> {
    let service = state
        .image_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Image service not found"
        )))?;

    service.set_expiry(id, auth_user.id, req.expires_at).await?;
    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(())
}

pub async fn delete_images(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(req): Json<DeleteRequest>,
) -> Result<(), AppError> {
    let service = state
        .image_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Image service not found"
        )))?;

    service
        .delete_images(&req.image_ids, auth_user.id, req.permanent)
        .await?;
    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(())
}

pub async fn get_deleted_images(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Paginated<Image>>, AppError> {
    let service = state
        .image_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Image service not found"
        )))?;

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    let images = service.get_deleted_images(auth_user.id).await?;
    let total = images.len() as i64;
    let start = ((page - 1) * page_size) as usize;
    let end = std::cmp::min(start + page_size as usize, images.len());
    let data = if start < images.len() {
        images[start..end].to_vec()
    } else {
        Vec::new()
    };
    let has_next = ((page * page_size) as i64) < total;

    Ok(Json(Paginated {
        data,
        page,
        page_size,
        total,
        has_next,
    }))
}

pub async fn restore_images(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(req): Json<RestoreRequest>,
) -> Result<(), AppError> {
    let service = state
        .image_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Image service not found"
        )))?;

    service.restore_images(&req.image_ids, auth_user.id).await?;
    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(())
}

pub async fn duplicate_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(_id): Path<Uuid>,
    Json(req): Json<DuplicateRequest>,
) -> Result<Json<Image>, AppError> {
    let service = state
        .image_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Image service not found"
        )))?;

    let duplicated = service
        .duplicate_image_v2(req.image_id, auth_user.id)
        .await?;
    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(Json(duplicated))
}

pub async fn edit_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<EditImageRequest>,
) -> Result<Json<EditImageResponse>, AppError> {
    let service = state
        .image_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Image service not found"
        )))?;

    let response = service.edit_image(id, auth_user.id, req).await?;
    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(Json(response))
}
