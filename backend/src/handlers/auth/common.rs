use crate::config::CookieConfig;
use crate::db::AppState;
use crate::domain::auth::DefaultAuthDomainService;
use crate::domain::auth::state_repository::{AuthStateRepository, AuthStateSnapshot, hash_token};
use crate::error::AppError;
use axum::http::{HeaderMap, HeaderValue, header};
use axum_extra::extract::cookie::CookieJar;
use chrono::{Duration, Utc};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::sync::Arc;

pub(super) const AUTH_TOKEN_COOKIE_NAME: &str = "auth_token";
pub(super) const REFRESH_TOKEN_COOKIE_NAME: &str = "refresh_token";

#[derive(Debug, Clone)]
pub(super) struct MailRuntimeConfig {
    pub link_base_url: String,
    smtp_host: String,
    smtp_port: u16,
    smtp_user: Option<String>,
    smtp_password: Option<String>,
    from_email: String,
    from_name: Option<String>,
}

fn optional_mailbox_name(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

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

pub(crate) fn append_session_cookies(
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
    CookieJar::from_headers(headers)
        .get(name)
        .map(|cookie| cookie.value().to_string())
}

pub(super) async fn load_auth_state_snapshot(
    state: &AppState,
    user_id: uuid::Uuid,
) -> Result<AuthStateSnapshot, AppError> {
    let user_token_version = state
        .auth_state_repository
        .get_user_token_version(user_id)
        .await?
        .unwrap_or(0);
    let session_epoch = state.auth_state_repository.get_session_epoch().await?;

    Ok(AuthStateSnapshot {
        user_token_version,
        session_epoch,
    })
}

pub(crate) async fn issue_session_tokens(
    state: &AppState,
    user_id: uuid::Uuid,
    email: &str,
    role: &str,
) -> Result<(String, String), AppError> {
    let snapshot = load_auth_state_snapshot(state, user_id).await?;
    let access_token = state.auth.generate_access_token(
        user_id,
        email,
        role,
        snapshot.user_token_version,
        snapshot.session_epoch,
    )?;
    let refresh_token = state.auth.generate_refresh_token(
        user_id,
        snapshot.user_token_version,
        snapshot.session_epoch,
    )?;
    Ok((access_token, refresh_token))
}

pub(super) async fn revoke_token(state: &AppState, token: &str) -> Result<(), AppError> {
    let ttl_seconds = state.auth.token_ttl_seconds(token).unwrap_or(0);
    if ttl_seconds == 0 {
        return Ok(());
    }

    let expires_at = Utc::now() + Duration::seconds(ttl_seconds.min(i64::MAX as u64) as i64);
    state
        .auth_state_repository
        .revoke_token_hash(&hash_token(token), expires_at)
        .await?;
    Ok(())
}

pub(super) fn append_query_params(base_url: &str, params: &[(&str, &str)]) -> String {
    let separator = if base_url.contains('?') { '&' } else { '?' };
    let query = params
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");
    format!("{base_url}{separator}{query}")
}

pub(super) async fn load_mail_runtime_config(
    state: &AppState,
) -> Result<MailRuntimeConfig, AppError> {
    let settings = state.runtime_settings.get_runtime_settings().await?;
    if !settings.mail_enabled {
        return Err(AppError::MailServiceNotEnabled);
    }

    Ok(MailRuntimeConfig {
        link_base_url: settings.mail_link_base_url,
        smtp_host: settings.mail_smtp_host,
        smtp_port: settings.mail_smtp_port,
        smtp_user: settings.mail_smtp_user,
        smtp_password: settings.mail_smtp_password,
        from_email: settings.mail_from_email,
        from_name: optional_mailbox_name(&settings.mail_from_name),
    })
}

pub(super) async fn send_text_mail_with_config(
    mail: &MailRuntimeConfig,
    recipient_email: &str,
    recipient_name: &str,
    subject: &str,
    body: String,
) -> Result<(), AppError> {
    let from = Mailbox::new(
        mail.from_name.clone(),
        mail.from_email.parse().map_err(|error| {
            AppError::Internal(anyhow::anyhow!("invalid from email: {}", error))
        })?,
    );
    let to = Mailbox::new(
        optional_mailbox_name(recipient_name),
        recipient_email.parse().map_err(|error| {
            AppError::Internal(anyhow::anyhow!("invalid recipient email: {}", error))
        })?,
    );
    let email = Message::builder()
        .from(from)
        .to(to)
        .subject(subject)
        .body(body)
        .map_err(|error| AppError::Internal(anyhow::anyhow!("build mail failed: {}", error)))?;

    let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&mail.smtp_host)
        .port(mail.smtp_port);
    if let (Some(user), Some(password)) = (mail.smtp_user.clone(), mail.smtp_password.clone()) {
        builder = builder.credentials(Credentials::new(user, password));
    }

    builder
        .build()
        .send(email)
        .await
        .map_err(|error| AppError::Internal(anyhow::anyhow!("send mail failed: {}", error)))?;

    Ok(())
}

pub(super) async fn ensure_app_installed(state: &AppState) -> Result<(), AppError> {
    if crate::db::is_app_installed(&state.database).await? {
        Ok(())
    } else {
        Err(AppError::AppNotInstalled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_cookie_returns_named_cookie_value() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("foo=bar; auth_token=hello%20world"),
        );

        assert_eq!(
            read_cookie(&headers, AUTH_TOKEN_COOKIE_NAME).as_deref(),
            Some("hello world")
        );
    }

    #[test]
    fn read_cookie_returns_none_when_cookie_is_missing() {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, HeaderValue::from_static("foo=bar"));

        assert_eq!(read_cookie(&headers, AUTH_TOKEN_COOKIE_NAME), None);
    }
}
