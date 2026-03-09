use super::*;

impl<I: ImageRepository, C: CategoryRepository> ImageDomainService<I, C> {
    /// Cursor-based 图片分页
    #[tracing::instrument(skip(self))]
    pub async fn get_images_cursor(
        &self,
        user_id: Uuid,
        params: crate::models::PaginationParams,
    ) -> Result<crate::models::CursorPaginated<Image>, AppError> {
        let limit = params.page_size.unwrap_or(20).clamp(1, 100);

        let cursor = match (params.cursor_created_at, params.cursor_id, params.cursor) {
            (Some(time), Some(id), _) => Some((time, id)),
            (Some(_), None, _) | (None, Some(_), _) => return Err(AppError::InvalidPagination),
            (_, _, Some((time, id_str))) => {
                let id = Uuid::parse_str(&id_str).map_err(|_| AppError::InvalidPagination)?;
                Some((time, id))
            }
            _ => None,
        };

        let images = self
            .image_repository
            .find_images_by_user_cursor(user_id, cursor, limit)
            .await?;

        let next_cursor = if images.len() == limit as usize {
            images
                .last()
                .map(|img| (img.created_at, img.id.to_string()))
        } else {
            None
        };

        Ok(crate::models::CursorPaginated {
            data: images,
            next_cursor,
        })
    }
}
