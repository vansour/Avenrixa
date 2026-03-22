use crate::db::AppState;
use crate::domain::image::DefaultImageDomainService;
use crate::error::AppError;
use crate::models::{CursorPaginated, Image, ImageResponse};
use std::sync::Arc;

pub(super) fn image_service(state: &AppState) -> Result<Arc<DefaultImageDomainService>, AppError> {
    Ok(state.image_domain_service.clone())
}

pub(super) fn map_cursor_images(result: CursorPaginated<Image>) -> CursorPaginated<ImageResponse> {
    CursorPaginated {
        data: result.data.into_iter().map(ImageResponse::from).collect(),
        limit: result.limit,
        next_cursor: result.next_cursor,
        has_next: result.has_next,
    }
}
