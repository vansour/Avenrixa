use super::common::image_service;
use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::models::{SetExpiryRequest, UpdateImageRequest};
use axum::{
    Json,
    extract::{Path, State},
};

pub async fn update_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(image_key): Path<String>,
    Json(req): Json<UpdateImageRequest>,
) -> Result<(), AppError> {
    let service = image_service(&state)?;

    service
        .update_image_tags_by_key(&image_key, auth_user.id, req.tags.as_deref())
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
    let service = image_service(&state)?;

    service
        .set_expiry_by_key(&image_key, auth_user.id, req.expires_at)
        .await?;
    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(())
}
