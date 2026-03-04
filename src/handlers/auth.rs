use crate::audit::log_audit;
use crate::auth::AuthService;
use crate::db::AppState;
use crate::email::MailService;
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::models::*;
use axum::{
    extract::State,
    Json,
};
use chrono::{Duration, Utc};
use tracing::{info, warn};
use uuid::Uuid;

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    if req.username.len() < 3 || req.username.len() > 50 {
        return Err(AppError::InvalidUsernameLength);
    }
    if req.password.len() < 6 {
        return Err(AppError::InvalidPasswordLength);
    }

    let password_hash = AuthService::hash_password(&req.password)?;

    let user_id = Uuid::new_v4();

    let res = sqlx::query(
        "INSERT INTO users (id, username, password_hash, role, created_at) VALUES ($1, $2, $3, 'user', NOW())"
    )
    .bind(user_id)
    .bind(&req.username)
    .bind(&password_hash)
    .execute(&state.pool)
    .await;

    if let Err(e) = res {
        if e.to_string().contains("duplicate key") {
            return Err(AppError::UsernameExists);
        }
        return Err(AppError::DatabaseError(e));
    }

    let user: User = sqlx::query_as(
        "SELECT id, username, password_hash, role, created_at FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await?;

    let access_token = state.auth.generate_access_token(user_id, &user.username, &user.role)?;
    let refresh_token = state.auth.generate_refresh_token(user_id)?;

    info!("User registered: {}", user.username);
    log_audit(&state.pool, Some(user_id), "user.register", "user", Some(user_id), None, None).await;
    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        expires_in: 900,  // 15分钟 = 900秒
        user: user.into(),
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<(axum::http::HeaderMap, Json<AuthResponse>), AppError> {
    let user: User = sqlx::query_as(
        "SELECT id, username, password_hash, role, created_at FROM users WHERE username = $1"
    )
    .bind(&req.username)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::InvalidPassword)?;

    let is_valid = AuthService::verify_password(&req.password, &user.password_hash)?;

    if !is_valid {
        return Err(AppError::InvalidPassword);
    }

    let access_token = state.auth.generate_access_token(user.id, &user.username, &user.role)?;
    let refresh_token = state.auth.generate_refresh_token(user.id)?;

    // 设置 httpOnly Cookie (存储 refresh_token)
    let cookie_value = format!(
        "auth_token={}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={}",
        refresh_token,
        7 * 24 * 3600  // 7天
    );

    let mut headers = axum::http::HeaderMap::new();
    headers.insert("Set-Cookie", axum::http::HeaderValue::from_str(&cookie_value).unwrap());

    info!("User logged in: {}", user.username);
    log_audit(&state.pool, Some(user.id), "user.login", "user", Some(user.id), None, None).await;
    Ok((headers, Json(AuthResponse {
        access_token,
        refresh_token,
        expires_in: 900,  // 15分钟 = 900秒
        user: user.into(),
    })))
}

pub async fn get_current_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<UserResponse, AppError> {
    let user: User = sqlx::query_as(
        "SELECT id, username, password_hash, role, created_at FROM users WHERE id = $1"
    )
    .bind(auth_user.id)
    .fetch_one(&state.pool)
    .await?;

    Ok(user.into())
}

pub async fn forgot_password(
    State(state): State<AppState>,
    Json(req): Json<ForgotPasswordRequest>,
) -> Result<(), AppError> {
    // 使用邮件服务发送密码重置链接

    // 查找用户（简化处理：使用 username 作为 email）
    let user: User = sqlx::query_as(
        "SELECT id, username, password_hash, role, created_at FROM users WHERE username = $1"
    )
    .bind(&req.email)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::UserNotFound)?;

    // 生成重置令牌
    let reset_token = crate::auth::AuthService::generate_reset_token();
    let expires_at = Utc::now() + Duration::hours(1); // 1小时有效
    let token_id = Uuid::new_v4();

    // 保存重置令牌
    sqlx::query(
        "INSERT INTO password_reset_tokens (id, user_id, token, expires_at, used_at, created_at) VALUES ($1, $2, $3, $4, NULL, NOW())"
    )
    .bind(token_id)
    .bind(user.id)
    .bind(&reset_token)
    .bind(expires_at)
    .execute(&state.pool)
    .await?;

    info!("Password reset token created for user: {}", user.username);

    // 发送密码重置邮件
    let mail_service = MailService::new(state.config.clone());
    mail_service.send_password_reset_email(&user.username, &reset_token)
        .await?;

    Ok(())
}

pub async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<(), AppError> {
    if req.new_password.len() < 6 || req.new_password.len() > 100 {
        return Err(AppError::InvalidPasswordLength);
    }

    // 查找有效的重置令牌
    let reset_data: (Uuid, Uuid, chrono::DateTime<Utc>) = sqlx::query_as(
        "SELECT id, user_id, expires_at FROM password_reset_tokens
         WHERE token = $1 AND used_at IS NULL AND expires_at > $2"
    )
    .bind(&req.token)
    .bind(Utc::now())
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::ResetTokenInvalid)?;

    let (token_id, user_id, _expires_at) = reset_data;

    // 检查令牌是否已使用
    let existing_used = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM password_reset_tokens WHERE id = $1 AND used_at IS NOT NULL)"
    )
    .bind(token_id)
    .fetch_one(&state.pool)
    .await?;

    if existing_used {
        return Err(AppError::ResetTokenExpired);
    }

    // 生成新密码哈希
    let new_hash = AuthService::hash_password(&req.new_password)?;

    // 更新密码
    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(&new_hash)
        .bind(user_id)
        .execute(&state.pool)
        .await?;

    // 标记令牌已使用
    sqlx::query("UPDATE password_reset_tokens SET used_at = $1 WHERE id = $2")
        .bind(Utc::now())
        .bind(token_id)
        .execute(&state.pool)
        .await?;

    info!("Password reset for user_id: {}", user_id);
    Ok(())
}

pub async fn logout(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<(axum::http::HeaderMap, ()), AppError> {
    info!("User logged out: {}", auth_user.username);
    log_audit(&state.pool, Some(auth_user.id), "user.logout", "user", Some(auth_user.id), None, None).await;

    // 清除 Cookie
    let cookie_value = "auth_token=; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=0";

    let mut headers = axum::http::HeaderMap::new();
    headers.insert("Set-Cookie", axum::http::HeaderValue::from_str(cookie_value).unwrap());

    Ok((headers, ()))
}

pub async fn change_password(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<(), AppError> {
    info!("change_password called for user: {}", auth_user.username);

    // 验证当前密码
    let user: User = sqlx::query_as(
        "SELECT id, username, password_hash, role, created_at FROM users WHERE id = $1"
    )
    .bind(auth_user.id)
    .fetch_one(&state.pool)
    .await?;

    let is_valid = AuthService::verify_password(&req.current_password, &user.password_hash)?;

    if !is_valid {
        warn!("Current password incorrect for user: {}", auth_user.username);
        return Err(AppError::InvalidPassword);
    }
    info!("Current password verified for user: {}", auth_user.username);

    // 获取新密码（如果有）
    if let Some(new_password) = req.new_password {
        if new_password.len() < 6 || new_password.len() > 100 {
            return Err(AppError::InvalidPasswordLength);
        }

        // 计算新密码哈希
        let new_hash = AuthService::hash_password(&new_password)?;

        // 更新密码
        sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
            .bind(&new_hash)
            .bind(auth_user.id)
            .execute(&state.pool)
            .await?;

        info!("Password changed for user_id: {}", auth_user.id);
        log_audit(&state.pool, Some(auth_user.id), "user.password_changed", "user", Some(auth_user.id), None, None).await;
    }

    Ok(())
}

/// 刷新访问令牌
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // 验证刷新令牌
    let user_id = state.auth.verify_refresh_token(&req.refresh_token)?;

    // 查找用户信息
    let user: User = sqlx::query_as(
        "SELECT id, username, password_hash, role, created_at FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await?;

    // 生成新的访问令牌和刷新令牌
    let access_token = state.auth.generate_access_token(user_id, &user.username, &user.role)?;
    let new_refresh_token = state.auth.generate_refresh_token(user_id)?;

    info!("Token refreshed for user: {}", user.username);
    Ok(Json(AuthResponse {
        access_token,
        refresh_token: new_refresh_token,
        expires_in: 900,  // 15分钟 = 900秒
        user: user.into(),
    }))
}
