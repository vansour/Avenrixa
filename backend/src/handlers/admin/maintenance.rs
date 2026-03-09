use super::common::admin_service;
use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AdminUser;
use crate::models::BackupResponse;
use axum::{Json, extract::State};

pub async fn cleanup_deleted_files(
    State(state): State<AppState>,
    admin_user: AdminUser,
) -> Result<Json<Vec<String>>, AppError> {
    let service = admin_service(&state)?;
    let removed = service
        .cleanup_deleted_files(admin_user.id, &admin_user.username)
        .await?;
    Ok(Json(removed))
}

pub async fn cleanup_expired_images(
    State(state): State<AppState>,
    admin_user: AdminUser,
) -> Result<Json<i64>, AppError> {
    let service = admin_service(&state)?;
    let affected = service.cleanup_expired_images(admin_user.id).await?;
    Ok(Json(affected))
}

pub async fn backup_database(
    State(state): State<AppState>,
    admin_user: AdminUser,
) -> Result<Json<BackupResponse>, AppError> {
    let service = admin_service(&state)?;
    let response = service
        .backup_database(admin_user.id, &admin_user.username)
        .await?;
    Ok(Json(response))
}
