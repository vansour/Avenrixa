use super::common::{image_service, map_paginated_images};
use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::models::{DeleteRequest, ImageResponse, Paginated, PaginationParams, RestoreRequest};
use axum::{
    Json,
    extract::{Query, State},
};

pub async fn delete_images(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(req): Json<DeleteRequest>,
) -> Result<(), AppError> {
    let service = image_service(&state)?;

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
    let service = image_service(&state)?;
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
    let service = image_service(&state)?;

    service
        .restore_images_by_keys(&req.image_keys, auth_user.id)
        .await?;
    let _ = state.invalidate_user_image_cache(auth_user.id).await;

    Ok(())
}
