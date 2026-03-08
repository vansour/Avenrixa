use crate::db::AppState;
use axum::http::StatusCode;
use redis::AsyncCommands;
use uuid::Uuid;

/// 认证用户信息提取器（只有一个管理员）
#[derive(Debug, Clone)]
pub struct AuthUser {
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
                .map_err(|_| StatusCode::UNAUTHORIZED)?
                .split(';')
                .find_map(|cookie| cookie.trim().strip_prefix("auth_token="))
                .ok_or(StatusCode::UNAUTHORIZED)?
        } else {
            // Cookie 中未找到，尝试从 Authorization header 获取
            parts
                .headers
                .get("authorization")
                .and_then(|h| h.to_str().ok())
                .and_then(|auth| auth.strip_prefix("Bearer "))
                .ok_or(StatusCode::UNAUTHORIZED)?
        };

        // 验证 JWT 令牌
        let claims = state
            .auth
            .verify_token(token)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        // 检查令牌是否已被撤销
        let mut redis = state.redis.clone();
        let revoked_key = format!("token_revoked:{}", token);
        let is_revoked: bool = redis
            .exists(&revoked_key)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if is_revoked {
            return Err(StatusCode::UNAUTHORIZED);
        }

        // 检查用户是否被标记为需要重新认证
        let user_revoked_key = format!("user_revoked:{}", claims.sub);
        let is_user_revoked: bool = redis
            .exists(&user_revoked_key)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if is_user_revoked {
            return Err(StatusCode::UNAUTHORIZED);
        }

        Ok(AuthUser {
            id: claims.sub,
            username: claims.username,
            role: claims.role,
        })
    }
}

/// 管理员用户信息提取器
#[derive(Debug, Clone)]
pub struct AdminUser {
    pub id: Uuid,
    pub username: String,
    pub role: String,
}

// AdminUser 提取器 - 验证用户角色为 admin
impl axum::extract::FromRequestParts<AppState> for AdminUser {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // 使用 AuthUser 提取器验证身份
        let auth_user = AuthUser::from_request_parts(parts, state).await?;

        // 验证用户角色为 admin
        if auth_user.role != "admin" {
            return Err(StatusCode::FORBIDDEN);
        }

        Ok(AdminUser {
            id: auth_user.id,
            username: auth_user.username,
            role: auth_user.role,
        })
    }
}
