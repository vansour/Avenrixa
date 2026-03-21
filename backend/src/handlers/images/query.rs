use super::common::{image_service, map_cursor_images};
use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::models::{CursorPaginated, CursorPaginationParams, ImageResponse};
use axum::{
    Json,
    extract::{Path, Query, State},
};

pub async fn get_images(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(params): Query<CursorPaginationParams>,
) -> Result<Json<CursorPaginated<ImageResponse>>, AppError> {
    let service = image_service(&state)?;
    let limit = params.limit.unwrap_or(20).clamp(1, 100);

    let result = service
        .get_images(auth_user.id, params.cursor.as_deref(), limit)
        .await?;

    Ok(Json(map_cursor_images(result)))
}

pub async fn get_image(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(image_key): Path<String>,
) -> Result<Json<ImageResponse>, AppError> {
    let service = image_service(&state)?;
    let image = service
        .increment_views_by_key(&image_key, auth_user.id)
        .await?;
    Ok(Json(ImageResponse::from(image)))
}
