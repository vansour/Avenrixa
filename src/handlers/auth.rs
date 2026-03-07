//! 认证相关 HTTP 处理器
//!
//! 仅处理 HTTP 请求/响应，业务逻辑委托给 AuthDomainService

use crate::audit::log_audit;
use crate::db::AppState;
use crate::db::ADMIN_USER_ID;
use crate::middleware::AuthUser;
use crate::models::*;
use crate::error::AppError;
use axum::{
    extract::State,
    Json,
};
use tracing::info;

/// 用户登录
#[tracing::instrument(skip(state, req), fields(username = %req.username))]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<(axum::http::HeaderMap, Json<UserResponse>), AppError> {
    // 使用领域服务处理登录逻辑
    let auth_domain_service = state.auth_domain_service.as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Auth domain service not initialized")))?;

    let (user, refresh_token) = auth_domain_service.login(req).await?;

    // 设置 httpOnly Cookie (存储 refresh_token)
    let cookie_value = format!(
        "auth_token={}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={}",
        refresh_token,
        7 * 24 * 3600  // 7天
    );

    let mut headers = axum::http::HeaderMap::new();
    headers.insert("Set-Cookie", axum::http::HeaderValue::from_str(&cookie_value).unwrap());

    // 记录审计日志
    log_audit(&state.pool, Some(ADMIN_USER_ID), "user.login", "user", Some(user.id), None, None).await;

    Ok((headers, Json(user)))
}

/// 获取当前用户信息
pub async fn get_current_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<UserResponse, AppError> {
    let auth_domain_service = state.auth_domain_service.as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Auth domain service not initialized")))?;

    auth_domain_service.get_current_user(auth_user.id).await
}

/// 用户登出
pub async fn logout(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<(axum::http::HeaderMap, ()), AppError> {
    info!("User logged out: {}", auth_user.username);

    // 记录审计日志
    log_audit(&state.pool, Some(ADMIN_USER_ID), "user.logout", "user", Some(ADMIN_USER_ID), None, None).await;

    // 清除 Cookie
    let cookie_value = "auth_token=; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=0";

    let mut headers = axum::http::HeaderMap::new();
    headers.insert("Set-Cookie", axum::http::HeaderValue::from_str(cookie_value).unwrap());

    Ok((headers, ()))
}

/// 修改密码
pub async fn change_password(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<(), AppError> {
    let auth_domain_service = state.auth_domain_service.as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Auth domain service not initialized")))?;

    auth_domain_service.change_password(auth_user.id, req).await?;

    // 记录审计日志
    log_audit(&state.pool, Some(ADMIN_USER_ID), "user.password_changed", "user", Some(ADMIN_USER_ID), None, None).await;

    Ok(())
}
