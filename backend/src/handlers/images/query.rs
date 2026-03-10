use super::common::{image_service, map_paginated_images};
use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::models::{ImageResponse, Paginated, PaginationParams};
use axum::{
    Json,
    extract::{Path, Query, State},
};

pub async fn get_images(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Paginated<ImageResponse>>, AppError> {
    let service = image_service(&state)?;
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    let result = service
        .get_images(auth_user.id, page, page_size, params.tag.as_deref())
        .await?;

    Ok(Json(map_paginated_images(result)))
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
