use axum::http::StatusCode;
use uuid::Uuid;
use crate::db::AppState;

/// 认证用户信息提取器
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub username: String,
    pub role: String,
}

/// 管理员用户信息提取器（自动检查角色）
#[derive(Debug, Clone)]
pub struct AdminUser {
    pub id: Uuid,
    pub username: String,
    pub role: String,
}

// AuthUser 提取器 - 使用 TypedHeader 和 State 提取器
impl axum::extract::FromRequestParts<AppState> for AuthUser {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // 优先从 Cookie 获取 token
        let token = if let Some(cookie_header) = parts.headers.get("cookie") {
            // 解析 Cookie header 查找 auth_token
            cookie_header
                .to_str()
                .map_err(|_| StatusCode::BAD_REQUEST)?
                .split(';')
                .find_map(|cookie| cookie.trim().strip_prefix("auth_token="))
                .ok_or(StatusCode::BAD_REQUEST)?
        } else {
            // Cookie 中未找到，尝试从 Authorization header 获取
            parts
                .headers
                .get("authorization")
                .and_then(|h| h.to_str().ok())
                .and_then(|auth| auth.strip_prefix("Bearer "))
                .ok_or(StatusCode::BAD_REQUEST)?
        };

        // 验证 JWT 令牌
        let claims = state.auth.verify_token(token)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        Ok(AuthUser {
            id: claims.sub,
            username: claims.username,
            role: claims.role,
        })
    }
}

// AdminUser 提取器 - 使用 AuthUser 并检查管理员角色
impl axum::extract::FromRequestParts<AppState> for AdminUser {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // 使用 AuthUser 提取器
        let user = AuthUser::from_request_parts(parts, state).await?;

        // 检查管理员角色
        if user.role != "admin" {
            return Err(StatusCode::FORBIDDEN);
        }

        Ok(AdminUser {
            id: user.id,
            username: user.username,
            role: user.role,
        })
    }
}

/// JWT 认证中间件 - 保留供将来使用（已禁用）
#[allow(dead_code)]
pub async fn auth_middleware(
    state: &AppState,
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    // 跳过健康检查端点
    if req.uri().path() == "/health" {
        return Ok(next.run(req).await);
    }

    // 检查 Authorization header
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 验证 Bearer token 格式
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 验证 JWT 令牌
    let claims = state.auth.verify_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // 将用户信息添加到 request extensions
    req.extensions_mut().insert(AuthUser {
        id: claims.sub,
        username: claims.username,
        role: claims.role,
    });

    Ok(next.run(req).await)
}

/// 管理员认证中间件 - 保留供将来使用（已禁用）
#[allow(dead_code)]
pub async fn admin_auth_middleware(
    state: &AppState,
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    // 跳过健康检查端点
    if req.uri().path() == "/health" {
        return Ok(next.run(req).await);
    }

    // 检查 Authorization header
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 验证 Bearer token 格式
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 验证 JWT 令牌
    let claims = state.auth.verify_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // 检查管理员角色
    if claims.role != "admin" {
        return Err(StatusCode::FORBIDDEN);
    }

    // 将用户信息添加到 request extensions
    req.extensions_mut().insert(AuthUser {
        id: claims.sub,
        username: claims.username,
        role: claims.role,
    });

    Ok(next.run(req).await)
}
