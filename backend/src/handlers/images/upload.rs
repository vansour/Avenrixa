use super::common::image_service;
use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::models::ImageResponse;
use axum::{Json, extract::State};
use axum_extra::extract::Multipart;
use tokio::io::AsyncWriteExt;
use tracing::{error, warn};
use uuid::Uuid;

#[tracing::instrument(skip(state, auth_user, multipart), fields(user_id = %auth_user.id))]
pub async fn upload_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<ImageResponse>, AppError> {
    let service = image_service(&state)?;

    while let Some(field) = multipart.next_field().await.map_err(|_| {
        error!("Failed to read multipart field");
        AppError::InvalidImageFormat
    })? {
        let name = field.name().unwrap_or("");
        if name == "file" {
            let filename = field.file_name().unwrap_or("unknown").to_string();
            let content_type = field.content_type().map(|ct| ct.to_string());

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
