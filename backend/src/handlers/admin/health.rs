use super::common::admin_service;
use crate::db::AppState;
use crate::error::AppError;
use crate::models::HealthStatus;
use axum::{Json, extract::State};

#[tracing::instrument(skip(state))]
pub async fn health_check(State(state): State<AppState>) -> Result<Json<HealthStatus>, AppError> {
    let service = admin_service(&state)?;
    let status = service
        .health_check(state.started_at.elapsed().as_secs())
        .await?;
    Ok(Json(status))
}
