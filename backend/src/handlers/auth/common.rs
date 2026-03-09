use crate::config::CookieConfig;
use crate::db::AppState;
use crate::domain::auth::DefaultAuthDomainService;
use crate::error::AppError;
use axum::http::{HeaderMap, HeaderValue, header};
use std::sync::Arc;

pub(super) const AUTH_TOKEN_COOKIE_NAME: &str = "auth_token";
pub(super) const REFRESH_TOKEN_COOKIE_NAME: &str = "refresh_token";

pub(super) fn auth_domain_service(
    state: &AppState,
) -> Result<Arc<DefaultAuthDomainService>, AppError> {
    state
        .auth_domain_service
        .clone()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Auth domain service not initialized")))
}

pub(super) fn build_cookie(
    cookie_config: &CookieConfig,
    name: &str,
    token: &str,
    max_age: u64,
) -> Result<HeaderValue, AppError> {
    let mut cookie = format!(
        "{}={}; HttpOnly; SameSite={}; Path={}; Max-Age={}",
        name, token, cookie_config.same_site, cookie_config.path, max_age
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

pub(super) fn append_session_cookies(
    headers: &mut HeaderMap,
    cookie_config: &CookieConfig,
    access_token: &str,
    access_max_age: u64,
    refresh_token: &str,
    refresh_max_age: u64,
) -> Result<(), AppError> {
    headers.append(
        header::SET_COOKIE,
        build_cookie(
            cookie_config,
            AUTH_TOKEN_COOKIE_NAME,
            access_token,
            access_max_age,
        )?,
    );
    headers.append(
        header::SET_COOKIE,
        build_cookie(
            cookie_config,
            REFRESH_TOKEN_COOKIE_NAME,
            refresh_token,
            refresh_max_age,
        )?,
    );
    Ok(())
}

pub(super) fn append_cleared_session_cookies(
    headers: &mut HeaderMap,
    cookie_config: &CookieConfig,
) -> Result<(), AppError> {
    headers.append(
        header::SET_COOKIE,
        build_cookie(cookie_config, AUTH_TOKEN_COOKIE_NAME, "", 0)?,
    );
    headers.append(
        header::SET_COOKIE,
        build_cookie(cookie_config, REFRESH_TOKEN_COOKIE_NAME, "", 0)?,
    );
    Ok(())
}

pub(super) fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|cookie| {
            let (key, value) = cookie.trim().split_once('=')?;
            if key == name {
                Some(value.to_string())
            } else {
                None
            }
        })
}
