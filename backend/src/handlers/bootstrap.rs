use axum::{Json, extract::State};

use crate::bootstrap::BootstrapAppState;
use crate::db::AppState;
use crate::error::AppError;
use crate::models::{
    BootstrapStatusResponse, ComponentStatus, HealthState, HealthStatus,
    UpdateBootstrapDatabaseConfigRequest, UpdateBootstrapDatabaseConfigResponse,
};

pub async fn get_bootstrap_status(
    State(state): State<BootstrapAppState>,
) -> Result<Json<BootstrapStatusResponse>, AppError> {
    let file = state.store.load().await.map_err(AppError::Internal)?;
    Ok(Json(state.store.bootstrap_status(
        &state.config,
        file.as_ref(),
        state.runtime_error.clone(),
    )))
}

pub async fn update_bootstrap_database_config(
    State(state): State<BootstrapAppState>,
    Json(req): Json<UpdateBootstrapDatabaseConfigRequest>,
) -> Result<Json<UpdateBootstrapDatabaseConfigResponse>, AppError> {
    let response = state
        .store
        .save_database_config(&req, state.config.database.max_connections)
        .await
        .map_err(|error| AppError::ValidationError(error.to_string()))?;
    Ok(Json(response))
}

pub async fn bootstrap_health_check(
    State(state): State<BootstrapAppState>,
) -> Result<Json<HealthStatus>, AppError> {
    let file = state.store.load().await.map_err(AppError::Internal)?;
    let database_message = match (&file, &state.runtime_error) {
        (_, Some(error)) => Some(format!("数据库连接失败: {}", error)),
        (Some(_), None) => Some("数据库配置已保存，重启服务后继续安装".to_string()),
        (None, None) => Some("尚未配置数据库".to_string()),
    };

    Ok(Json(HealthStatus {
        status: HealthState::Bootstrapping,
        timestamp: chrono::Utc::now(),
        database: ComponentStatus {
            status: HealthState::Unhealthy,
            message: database_message,
        },
        cache: ComponentStatus {
            status: HealthState::Disabled,
            message: Some("Bootstrap 模式未启用外部缓存".to_string()),
        },
        storage: ComponentStatus::healthy(),
        observability: ComponentStatus {
            status: HealthState::Disabled,
            message: Some("Bootstrap 模式尚未启用运行态指标".to_string()),
        },
        version: None,
        uptime_seconds: Some(state.started_at.elapsed().as_secs()),
        metrics: None,
    }))
}

pub async fn get_runtime_bootstrap_status(
    State(state): State<AppState>,
) -> Result<Json<BootstrapStatusResponse>, AppError> {
    let store = crate::bootstrap::BootstrapConfigStore::from_env();
    Ok(Json(store.runtime_status(&state.config)))
}

pub async fn reject_runtime_database_config_update(
    State(_state): State<AppState>,
    Json(_req): Json<UpdateBootstrapDatabaseConfigRequest>,
) -> Result<Json<UpdateBootstrapDatabaseConfigResponse>, AppError> {
    Err(AppError::ValidationError(
        "数据库运行时已经初始化；如果需要修改数据库配置，请更新 bootstrap 配置并重启服务"
            .to_string(),
    ))
}
