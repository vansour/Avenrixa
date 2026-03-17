use super::common::{
    append_query_params, auth_domain_service, ensure_app_installed, load_mail_runtime_config,
    send_text_mail_with_config,
};
use crate::audit::log_audit_db;
use crate::db::AppState;
use crate::error::AppError;
use crate::models::{EmailVerificationConfirmRequest, RegisterRequest};
use axum::{Json, extract::State};

fn verification_link(base_url: &str, token: &str) -> String {
    append_query_params(base_url, &[("mode", "verify-email"), ("token", token)])
}

async fn send_verification_mail(
    state: &AppState,
    recipient_email: &str,
    recipient_name: &str,
    token: &str,
) -> Result<(), AppError> {
    let mail = load_mail_runtime_config(state).await?;
    let verify_url = verification_link(&mail.link_base_url, token);
    let body = format!(
        "你好，{}\n\n请访问以下链接完成邮箱验证：\n{}\n\n如果不是你本人注册，请忽略这封邮件。",
        recipient_name, verify_url
    );
    send_text_mail_with_config(
        &mail,
        recipient_email,
        recipient_name,
        "Avenrixa 邮箱验证",
        body,
    )
    .await
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(), AppError> {
    ensure_app_installed(&state).await?;
    let auth_domain_service = auth_domain_service(&state)?;
    let dispatch = auth_domain_service.register(req).await?;
    send_verification_mail(&state, &dispatch.email, &dispatch.email, &dispatch.token).await?;

    log_audit_db(
        &state.database,
        Some(dispatch.user_id),
        "user.register_requested",
        "user",
        Some(dispatch.user_id),
        None,
        None,
    )
    .await;

    Ok(())
}

pub async fn verify_registration_email(
    State(state): State<AppState>,
    Json(req): Json<EmailVerificationConfirmRequest>,
) -> Result<(), AppError> {
    ensure_app_installed(&state).await?;
    let auth_domain_service = auth_domain_service(&state)?;
    let user = auth_domain_service.verify_email(&req.token).await?;

    log_audit_db(
        &state.database,
        Some(user.id),
        "user.email_verified",
        "user",
        Some(user.id),
        None,
        None,
    )
    .await;

    Ok(())
}
