use super::common::image_service;
use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::models::DeleteRequest;
use axum::{Json, extract::State};

pub async fn delete_images(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(req): Json<DeleteRequest>,
) -> Result<(), AppError> {
    let service = image_service(&state)?;

    service
        .delete_images_by_keys(&req.image_keys, auth_user.id)
        .await?;
    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(())
}
