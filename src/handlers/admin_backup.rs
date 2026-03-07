//! 图片相关 HTTP 处理器备份
//!
//! 处理 HTTP 请求/响应，业务逻辑委托给 DomainService

use std::sync::Arc;

use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::{AdminUser, AuthUser};
use crate::models::*;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use redis::AsyncCommands;
use tracing::{error, info};
use uuid::Uuid;
use crate::audit::log_audit;

/// 获取 ImageDomainService 或返回错误
fn get_image_domain_service(state: &AppState) -> Result<Arc<ImageDomainService>, AppError> {
    state.image_domain_service.clone()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Image domain service not initialized")))
}

/// 将 HashCheckResult 转换为 handler 内部使用的结构
fn hash_result_to_info(result: HashCheckResult) -> ImageInfo {
    ImageInfo {
        id: result.id,
        filename: result.filename,
        user_id: result.user_id,
    }
}

/// 获取 AdminDomainService 或返回错误
fn get_admin_domain_service(state: &AppState) -> Result<Arc<AdminDomainService>, AppError> {
    state.admin_domain_service.clone()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Admin domain service not initialized")))
}
