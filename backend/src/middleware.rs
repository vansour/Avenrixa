use crate::db::AppState;
use crate::domain::auth::Claims;
use crate::domain::auth::{auth_valid_after_key, user_token_version_key};
use axum::http::StatusCode;
use redis::AsyncCommands;
use uuid::Uuid;

fn token_version_is_revoked(claims: &Claims, current_version: Option<u64>) -> bool {
    current_version.is_some_and(|version| claims.token_version < version)
}

fn token_issued_before_cutoff(claims: &Claims, valid_after: Option<i64>) -> bool {
    crate::sqlite_restore::token_issued_before_cutoff(claims.iat, valid_after)
}

/// 认证用户信息提取器（只有一个管理员）
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub token: String,
}

// AuthUser 提取器 - 使用 TypedHeader 和 State 提取器
impl axum::extract::FromRequestParts<AppState> for AuthUser {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let installed = crate::db::is_app_installed(&state.database)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if !installed {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }

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

        // 刷新令牌不能用于业务接口访问
        if claims.role == "refresh" {
            return Err(StatusCode::UNAUTHORIZED);
        }

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

        // 版本化会话控制: 改密后提升版本，所有旧 token 自动失效
        let current_token_version: Option<u64> = redis
            .get(user_token_version_key(claims.sub))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if token_version_is_revoked(&claims, current_token_version) {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let auth_valid_after: Option<i64> = redis
            .get(auth_valid_after_key())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if token_issued_before_cutoff(&claims, auth_valid_after) {
            return Err(StatusCode::UNAUTHORIZED);
        }

        Ok(AuthUser {
            id: claims.sub,
            email: claims.email,
            role: claims.role,
            token: token.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_claims(token_version: u64) -> Claims {
        Claims {
            sub: Uuid::new_v4(),
            email: "tester@example.com".to_string(),
            role: "user".to_string(),
            token_version,
            exp: 0,
            iat: 0,
        }
    }

    #[test]
    fn token_version_matches_when_no_server_version() {
        assert!(!token_version_is_revoked(&sample_claims(0), None));
    }

    #[test]
    fn token_version_matches_current_server_version() {
        assert!(!token_version_is_revoked(&sample_claims(2), Some(2)));
    }

    #[test]
    fn token_version_rejects_outdated_token() {
        assert!(token_version_is_revoked(&sample_claims(1), Some(2)));
    }

    #[test]
    fn cutoff_rejects_older_token() {
        assert!(token_issued_before_cutoff(&sample_claims(1), Some(1)));
    }
}

/// 管理员用户信息提取器
#[derive(Debug, Clone)]
pub struct AdminUser {
    pub id: Uuid,
    pub email: String,
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
            email: auth_user.email,
        })
    }
}
