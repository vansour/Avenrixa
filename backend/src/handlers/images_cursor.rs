//! Cursor-based 图片分页查询模块
//! 支持 cursor-based 分页以提高性能
//! 使用完全参数化查询，避免 SQL 注入风险

use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::models::Image;
use axum::Json;
use axum::extract::{Query, State};

pub async fn get_images(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(params): Query<crate::models::PaginationParams>,
) -> Result<Json<crate::models::CursorPaginated<Image>>, AppError> {
    let service = state.image_domain_service.as_ref().ok_or(AppError::Internal(anyhow::anyhow!("Image service not found")))?;
    let result = service.get_images_cursor(auth_user.id, params).await?;
    Ok(Json(result))
}
