use crate::db::AppState;
use crate::domain::image::DefaultImageDomainService;
use crate::error::AppError;
use crate::models::{Image, ImageResponse, Paginated};
use std::sync::Arc;

pub(super) fn image_service(state: &AppState) -> Result<Arc<DefaultImageDomainService>, AppError> {
    state
        .image_domain_service
        .clone()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Image service not found"
        )))
}

pub(super) fn map_paginated_images(result: Paginated<Image>) -> Paginated<ImageResponse> {
    Paginated {
        data: result.data.into_iter().map(ImageResponse::from).collect(),
        page: result.page,
        page_size: result.page_size,
        total: result.total,
        has_next: result.has_next,
    }
}
