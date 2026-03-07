//! 认证相关 HTTP 处理器
//!
//! 仅处理 HTTP 请求/响应，业务逻辑委托给 AuthDomainService

use crate::audit::log_audit;
use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::models::*;
use axum::{
    extract::State,
    Json,
};
use tracing::info;

/// 用户注册
#[tracing::instrument(skip(state, req), fields(username = %req.username))]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // 使用领域服务处理注册逻辑
    let auth_domain_service = state.auth_domain_service.as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Auth domain service not initialized")))?;

    let response = auth_domain_service.register(req).await?;

    // 记录审计日志
    log_audit(&state.pool, Some(response.user.id), "user.register", "user", Some(response.user.id), None, None).await;

    Ok(Json(response))
}

/// 用户登录
#[tracing::instrument(skip(state, req), fields(username = %req.username))]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<(axum::http::HeaderMap, Json<AuthResponse>), AppError> {
    // 使用领域服务处理登录逻辑
    let auth_domain_service = state.auth_domain_service.as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Auth domain service not initialized")))?;

    let (response, refresh_token) = auth_domain_service.login(req).await?;

    // 设置 httpOnly Cookie (存储 refresh_token)
    let cookie_value = format!(
        "auth_token={}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={}",
        refresh_token,
        7 * 24 * 3600  // 7天
    );

    let mut headers = axum::http::HeaderMap::new();
    headers.insert("Set-Cookie", axum::http::HeaderValue::from_str(&cookie_value).unwrap());

    // 记录审计日志
    log_audit(&state.pool, Some(response.user.id), "user.login", "user", Some(response.user.id), None, None).await;

    Ok((headers, Json(response)))
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

/// 忘记密码
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(req): Json<ForgotPasswordRequest>,
) -> Result<(), AppError> {
    let auth_domain_service = state.auth_domain_service.as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Auth domain service not initialized")))?;

    // 领域服务会自动处理邮件发送（如果已配置）
    auth_domain_service.forgot_password(req.email).await
}

/// 重置密码
pub async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<(), AppError> {
    let auth_domain_service = state.auth_domain_service.as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Auth domain service not initialized")))?;

    auth_domain_service.reset_password(req.token, req.new_password).await
}

/// 用户登出
pub async fn logout(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<(axum::http::HeaderMap, ()), AppError> {
    info!("User logged out: {}", auth_user.username);

    // 记录审计日志
    log_audit(&state.pool, Some(auth_user.id), "user.logout", "user", Some(auth_user.id), None, None).await;

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

    // 获取新密码（如果有）
    if let Some(new_password) = req.new_password {
        auth_domain_service.change_password(
            auth_user.id,
            req.current_password,
            new_password,
        ).await?;

        // 记录审计日志
        log_audit(&state.pool, Some(auth_user.id), "user.password_changed", "user", Some(auth_user.id), None, None).await;
    }

    Ok(())
}

/// 刷新访问令牌
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let auth_domain_service = state.auth_domain_service.as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Auth domain service not initialized")))?;

    let response = auth_domain_service.refresh_token(req.refresh_token).await?;
    Ok(Json(response))
}
