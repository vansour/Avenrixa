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

fn map_paginated_images(result: Paginated<Image>) -> Paginated<ImageResponse> {
    Paginated {
        data: result.data.into_iter().map(ImageResponse::from).collect(),
        page: result.page,
        page_size: result.page_size,
        total: result.total,
        has_next: result.has_next,
    }
}

#[tracing::instrument(skip(state, auth_user, multipart), fields(user_id = %auth_user.id))]
pub async fn upload_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<ImageResponse>, AppError> {
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
            let _ = state.invalidate_user_image_cache(auth_user.id).await;

            return Ok(Json(ImageResponse::from(image)));
        }
    }

    warn!("No file field found in multipart");
    Err(AppError::InvalidImageFormat)
}

pub async fn get_images(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Paginated<ImageResponse>>, AppError> {
    let service = state
        .image_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Image service not found"
        )))?;

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    // 固定按上传时间倒序，不开放搜索和自定义排序。
    let result = service
        .get_images(
            auth_user.id,
            page,
            page_size,
            "created_at",
            "DESC",
            None,
            None,
            None,
        )
        .await?;

    Ok(Json(map_paginated_images(result)))
}

pub async fn get_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(image_key): Path<String>,
) -> Result<Json<ImageResponse>, AppError> {
    let service = state
        .image_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Image service not found"
        )))?;

    let image = service
        .increment_views_by_key(&image_key, auth_user.id)
        .await?;
    Ok(Json(ImageResponse::from(image)))
}

pub async fn update_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(image_key): Path<String>,
    Json(req): Json<UpdateCategoryRequest>,
) -> Result<(), AppError> {
    let service = state
        .image_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Image service not found"
        )))?;

    service
        .update_image_category_by_key(
            &image_key,
            auth_user.id,
            req.category_id,
            req.tags.as_deref(),
        )
        .await?;

    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(())
}

pub async fn set_expiry(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(image_key): Path<String>,
    Json(req): Json<SetExpiryRequest>,
) -> Result<(), AppError> {
    let service = state
        .image_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Image service not found"
        )))?;

    service
        .set_expiry_by_key(&image_key, auth_user.id, req.expires_at)
        .await?;
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
        .delete_images_by_keys(&req.image_keys, auth_user.id, req.permanent)
        .await?;
    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(())
}

pub async fn get_deleted_images(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Paginated<ImageResponse>>, AppError> {
    let service = state
        .image_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Image service not found"
        )))?;

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    let result = service
        .get_deleted_images_paginated(auth_user.id, page, page_size)
        .await?;

    Ok(Json(map_paginated_images(result)))
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

    service
        .restore_images_by_keys(&req.image_keys, auth_user.id)
        .await?;
    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(())
}

pub async fn edit_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(image_key): Path<String>,
    Json(req): Json<EditImageRequest>,
) -> Result<Json<EditImageResponse>, AppError> {
    let service = state
        .image_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Image service not found"
        )))?;

    let response = service
        .edit_image_by_key(&image_key, auth_user.id, req)
        .await?;
    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(Json(response))
}
