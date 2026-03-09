use super::common::admin_service;
use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AdminUser;
use crate::models::{AuditLogResponse, PaginationParams, SystemStats};
use axum::{
    Json,
    extract::{Query, State},
};

pub async fn get_audit_logs(
    State(state): State<AppState>,
    _admin_user: AdminUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<AuditLogResponse>, AppError> {
    let service = admin_service(&state)?;
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(50).clamp(1, 100);

    let response = service
        .get_audit_logs(page as i64, page_size as i64)
        .await?;
    Ok(Json(response))
}

pub async fn get_system_stats(
    State(state): State<AppState>,
    _admin_user: AdminUser,
) -> Result<Json<SystemStats>, AppError> {
    let service = admin_service(&state)?;
    let stats = service.get_system_stats().await?;
    Ok(Json(stats))
}
