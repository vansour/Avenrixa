use crate::db::AppState;
use crate::domain::auth::Claims;
use crate::domain::auth::state_repository::{AuthStateRepository, hash_token};
use crate::models::UserRole;
use axum::http::StatusCode;
use axum_extra::extract::cookie::CookieJar;
use uuid::Uuid;

fn token_version_mismatched(claims: &Claims, current_version: u64) -> bool {
    claims.token_version != current_version
}

fn session_epoch_mismatched(claims: &Claims, current_epoch: u64) -> bool {
    claims.session_epoch != current_epoch
}

/// 认证用户信息提取器（只有一个管理员）
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub role: UserRole,
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

        let cookie_jar = CookieJar::from_headers(&parts.headers);

        // 优先从 Cookie 获取 token，缺失时再回退到 Authorization header
        let token = cookie_jar
            .get("auth_token")
            .map(|cookie| cookie.value())
            .or_else(|| {
                parts
                    .headers
                    .get("authorization")
                    .and_then(|header| header.to_str().ok())
                    .and_then(|auth| auth.strip_prefix("Bearer "))
            })
            .ok_or(StatusCode::UNAUTHORIZED)?;

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
        let is_revoked = state
            .auth_state_repository
            .is_token_hash_revoked(&hash_token(token))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if is_revoked {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let snapshot = state
            .auth_state_repository
            .load_auth_snapshot(claims.sub)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::UNAUTHORIZED)?;

        if token_version_mismatched(&claims, snapshot.user_token_version) {
            return Err(StatusCode::UNAUTHORIZED);
        }

        if session_epoch_mismatched(&claims, snapshot.session_epoch) {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let role = UserRole::parse(&claims.role);
        if role == UserRole::Unknown {
            return Err(StatusCode::UNAUTHORIZED);
        }

        Ok(AuthUser {
            id: claims.sub,
            email: claims.email,
            role,
            token: token.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_claims(token_version: u64, session_epoch: u64) -> Claims {
        Claims {
            sub: Uuid::new_v4(),
            email: "tester@example.com".to_string(),
            role: UserRole::User.as_str().to_string(),
            token_version,
            session_epoch,
            exp: 0,
            iat: 0,
        }
    }

    #[test]
    fn token_version_matches_when_no_server_version() {
        assert!(!token_version_mismatched(&sample_claims(0, 0), 0));
    }

    #[test]
    fn token_version_matches_current_server_version() {
        assert!(!token_version_mismatched(&sample_claims(2, 0), 2));
    }

    #[test]
    fn token_version_rejects_outdated_token() {
        assert!(token_version_mismatched(&sample_claims(1, 0), 2));
    }

    #[test]
    fn session_epoch_matches_current_value() {
        assert!(!session_epoch_mismatched(&sample_claims(0, 2), 2));
    }

    #[test]
    fn session_epoch_rejects_mismatched_token() {
        assert!(session_epoch_mismatched(&sample_claims(0, 1), 2));
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
        if !auth_user.role.is_admin() {
            return Err(StatusCode::FORBIDDEN);
        }

        Ok(AdminUser {
            id: auth_user.id,
            email: auth_user.email,
        })
    }
}
