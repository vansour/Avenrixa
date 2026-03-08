//! 认证相关 HTTP 处理器
//!
//! 仅处理 HTTP 请求/响应，业务逻辑委托给 AuthDomainService

use crate::audit::log_audit;
use crate::config::CookieConfig;
use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::models::*;
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, HeaderValue, header},
};
use redis::AsyncCommands;
use tracing::info;

fn build_auth_cookie(
    cookie_config: &CookieConfig,
    token: &str,
    max_age: u64,
) -> Result<HeaderValue, AppError> {
    let mut cookie = format!(
        "auth_token={}; HttpOnly; SameSite={}; Path={}; Max-Age={}",
        token, cookie_config.same_site, cookie_config.path, max_age
    );

    if cookie_config.secure {
        cookie.push_str("; Secure");
    }

    if let Some(domain) = &cookie_config.domain {
        cookie.push_str("; Domain=");
        cookie.push_str(domain);
    }

    HeaderValue::from_str(&cookie).map_err(|e| {
        AppError::Internal(anyhow::anyhow!("Failed to build auth cookie header: {}", e))
    })
}

/// 用户登录
#[tracing::instrument(skip(state, req), fields(username = %req.username))]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<(axum::http::HeaderMap, Json<UserResponse>), AppError> {
    // 使用领域服务处理登录逻辑
    let auth_domain_service = state.auth_domain_service.as_ref().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("Auth domain service not initialized"))
    })?;

    let (user, access_token) = auth_domain_service.login(req).await?;

    // 用户重新登录成功后，清除用户级撤销标记
    let mut redis = state.redis.clone();
    let user_revoked_key = format!("user_revoked:{}", user.id);
    let _: Result<i32, _> = redis.del(user_revoked_key).await;

    // 设置 httpOnly Cookie（存储业务访问令牌）
    let cookie_header = build_auth_cookie(
        &state.config.cookie,
        &access_token,
        state.config.cookie.max_age_seconds,
    )?;

    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, cookie_header);

    // 记录审计日志
    log_audit(
        &state.pool,
        Some(user.id),
        "user.login",
        "user",
        Some(user.id),
        None,
        None,
    )
    .await;

    Ok((headers, Json(user)))
}

/// 获取当前用户信息
pub async fn get_current_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<UserResponse, AppError> {
    let auth_domain_service = state.auth_domain_service.as_ref().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("Auth domain service not initialized"))
    })?;

    auth_domain_service.get_current_user(auth_user.id).await
}

/// 用户登出
pub async fn logout(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<(axum::http::HeaderMap, ()), AppError> {
    info!("User logged out: {}", auth_user.username);

    // 撤销当前令牌（失效到其自然过期）
    let ttl = state
        .auth
        .token_ttl_seconds(&auth_user.token)
        .unwrap_or(7 * 24 * 3600)
        .max(1);
    let mut redis = state.redis.clone();
    let revoked_key = format!("token_revoked:{}", auth_user.token);
    let _: () = redis.set_ex(revoked_key, "1", ttl).await?;

    // 记录审计日志
    log_audit(
        &state.pool,
        Some(auth_user.id),
        "user.logout",
        "user",
        Some(auth_user.id),
        None,
        None,
    )
    .await;

    // 清除 Cookie（使用相同策略，Max-Age=0 让浏览器立即移除）
    let cookie_header = build_auth_cookie(&state.config.cookie, "", 0)?;
    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, cookie_header);

    Ok((headers, ()))
}

/// 修改密码
pub async fn change_password(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<(), AppError> {
    let auth_domain_service = state.auth_domain_service.as_ref().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("Auth domain service not initialized"))
    })?;

    auth_domain_service
        .change_password(auth_user.id, req)
        .await?;

    // 修改密码后撤销当前令牌，要求重新登录
    let ttl = state
        .auth
        .token_ttl_seconds(&auth_user.token)
        .unwrap_or(7 * 24 * 3600)
        .max(1);
    let mut redis = state.redis.clone();
    let revoked_key = format!("token_revoked:{}", auth_user.token);
    let _: () = redis.set_ex(revoked_key, "1", ttl).await?;

    // 记录审计日志
    log_audit(
        &state.pool,
        Some(auth_user.id),
        "user.password_changed",
        "user",
        Some(auth_user.id),
        None,
        None,
    )
    .await;

    Ok(())
}
