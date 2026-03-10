use super::common::admin_service;
use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AdminUser;
use crate::models::{
    BackupFileSummary, BackupResponse, BackupRestorePrecheckResponse,
    BackupRestoreScheduleResponse, BackupRestoreStatusResponse,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, header},
    response::{IntoResponse, Response},
};

pub async fn cleanup_deleted_files(
    State(state): State<AppState>,
    admin_user: AdminUser,
) -> Result<Json<Vec<String>>, AppError> {
    let service = admin_service(&state)?;
    let removed = service
        .cleanup_deleted_files(admin_user.id, &admin_user.email)
        .await?;
    Ok(Json(removed))
}

pub async fn cleanup_expired_images(
    State(state): State<AppState>,
    admin_user: AdminUser,
) -> Result<Json<i64>, AppError> {
    let service = admin_service(&state)?;
    let affected = service
        .cleanup_expired_images(admin_user.id, &admin_user.email)
        .await?;
    Ok(Json(affected))
}

pub async fn backup_database(
    State(state): State<AppState>,
    admin_user: AdminUser,
) -> Result<Json<BackupResponse>, AppError> {
    let service = admin_service(&state)?;
    let response = service
        .backup_database(admin_user.id, &admin_user.email)
        .await?;
    Ok(Json(response))
}

pub async fn get_backups(
    State(state): State<AppState>,
    _admin_user: AdminUser,
) -> Result<Json<Vec<BackupFileSummary>>, AppError> {
    let service = admin_service(&state)?;
    let backups = service.list_backups().await?;
    Ok(Json(backups))
}

pub async fn download_backup(
    Path(filename): Path<String>,
    State(state): State<AppState>,
    admin_user: AdminUser,
) -> Result<Response, AppError> {
    let service = admin_service(&state)?;
    let bytes = service
        .download_backup(admin_user.id, &admin_user.email, &filename)
        .await?;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename))
            .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))?,
    );
    headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&bytes.len().to_string())
            .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))?,
    );

    Ok((headers, bytes).into_response())
}

pub async fn delete_backup(
    Path(filename): Path<String>,
    State(state): State<AppState>,
    admin_user: AdminUser,
) -> Result<(), AppError> {
    let service = admin_service(&state)?;
    service
        .delete_backup(admin_user.id, &admin_user.email, &filename)
        .await
}

pub async fn get_restore_status(
    State(state): State<AppState>,
    _admin_user: AdminUser,
) -> Result<Json<BackupRestoreStatusResponse>, AppError> {
    let service = admin_service(&state)?;
    let status = service.get_restore_status().await?;
    Ok(Json(status))
}

pub async fn precheck_restore(
    Path(filename): Path<String>,
    State(state): State<AppState>,
    admin_user: AdminUser,
) -> Result<Json<BackupRestorePrecheckResponse>, AppError> {
    let service = admin_service(&state)?;
    let response = service
        .precheck_restore(admin_user.id, &admin_user.email, &filename)
        .await?;
    Ok(Json(response))
}

pub async fn schedule_restore(
    Path(filename): Path<String>,
    State(state): State<AppState>,
    admin_user: AdminUser,
) -> Result<Json<BackupRestoreScheduleResponse>, AppError> {
    let service = admin_service(&state)?;
    let response = service
        .schedule_restore(admin_user.id, &admin_user.email, &filename)
        .await?;
    Ok(Json(response))
}
