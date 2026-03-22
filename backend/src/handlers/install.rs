mod transactions;

use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};
use base64::Engine;
use shared_types::auth::UserResponse as SharedUserResponse;

use self::transactions::{
    InstallPersistenceInput, InstallRollbackInput, persist_installation,
    rollback_failed_installation,
};
use crate::audit::log_audit_db;
use crate::db::{
    AppState, INSTALL_STATE_SETTING_KEY, SITE_FAVICON_DATA_URL_SETTING_KEY, get_setting_value,
    has_admin_account, is_app_installed, validate_admin_bootstrap_config,
};
use crate::error::AppError;
use crate::handlers::auth::common::{append_session_cookies, issue_session_tokens};
use crate::handlers::storage_browser::{BrowseStorageDirectoriesQuery, browse_storage_directories};
use crate::models::storage_backend_kind_from_runtime;
use crate::models::{
    AdminSettingsConfig, InstallBootstrapRequest, InstallBootstrapResponse, InstallStatusResponse,
    StorageDirectoryBrowseResponse,
};
use crate::runtime_settings::RuntimeSettings;
use crate::runtime_settings::validate_and_merge;

fn runtime_settings_to_admin_config(
    settings: &RuntimeSettings,
    restart_required: bool,
) -> AdminSettingsConfig {
    AdminSettingsConfig {
        site_name: settings.site_name.clone(),
        storage_backend: storage_backend_kind_from_runtime(settings.storage_backend),
        local_storage_path: settings.local_storage_path.clone(),
        mail_enabled: settings.mail_enabled,
        mail_smtp_host: settings.mail_smtp_host.clone(),
        mail_smtp_port: settings.mail_smtp_port,
        mail_smtp_user: settings.mail_smtp_user.clone(),
        mail_smtp_password_set: settings
            .mail_smtp_password
            .as_ref()
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false),
        mail_from_email: settings.mail_from_email.clone(),
        mail_from_name: settings.mail_from_name.clone(),
        mail_link_base_url: settings.mail_link_base_url.clone(),
        restart_required,
        settings_version: settings.settings_version(),
    }
}

const MAX_FAVICON_BYTES: usize = 256 * 1024;

pub async fn get_install_status(
    State(state): State<AppState>,
) -> Result<Json<InstallStatusResponse>, AppError> {
    let installed = is_app_installed(&state.database).await?;
    let has_admin = has_admin_account(&state.database).await?;
    let settings = state.runtime_settings.get_runtime_settings().await?;
    let favicon_configured = get_setting_value(&state.database, SITE_FAVICON_DATA_URL_SETTING_KEY)
        .await?
        .is_some_and(|value| !value.trim().is_empty());
    let restart_required = if installed {
        state.storage_manager.restart_required(&settings)
    } else {
        false
    };

    Ok(Json(InstallStatusResponse {
        installed,
        has_admin,
        favicon_configured,
        config: public_install_status_config(&settings, installed, restart_required),
    }))
}

pub async fn browse_install_storage_directories(
    State(state): State<AppState>,
    Query(query): Query<BrowseStorageDirectoriesQuery>,
) -> Result<Json<StorageDirectoryBrowseResponse>, AppError> {
    if is_app_installed(&state.database).await? {
        return Err(AppError::AppAlreadyInstalled);
    }

    Ok(Json(
        browse_storage_directories(query.path.as_deref()).await?,
    ))
}

pub async fn bootstrap_installation(
    State(state): State<AppState>,
    Json(req): Json<InstallBootstrapRequest>,
) -> Result<(HeaderMap, Json<InstallBootstrapResponse>), AppError> {
    let _install_guard = state.installation_lock.lock().await;
    let InstallBootstrapRequest {
        admin_email,
        admin_password,
        favicon_data_url,
        config,
    } = req;

    let admin = validate_admin_bootstrap_config(admin_email, admin_password)
        .map_err(|error| AppError::ValidationError(error.to_string()))?;
    let favicon_data_url = validate_favicon_data_url(favicon_data_url)?;
    let current_settings = state.runtime_settings.get_runtime_settings().await?;
    let previous_install_state =
        get_setting_value(&state.database, INSTALL_STATE_SETTING_KEY).await?;
    let previous_favicon =
        get_setting_value(&state.database, SITE_FAVICON_DATA_URL_SETTING_KEY).await?;
    let validated_settings = validate_and_merge(current_settings.clone(), config)?;
    state
        .storage_manager
        .validate_runtime_settings(&validated_settings)
        .await?;

    let user = persist_installation(
        &state,
        &InstallPersistenceInput {
            validated_settings: &validated_settings,
            admin_email: &admin.email,
            admin_password: &admin.password,
            favicon_data_url: favicon_data_url.as_deref(),
        },
    )
    .await?;

    if let Err(apply_error) = state
        .storage_manager
        .apply_runtime_settings(validated_settings.clone())
        .await
    {
        let rollback_result = rollback_failed_installation(
            &state,
            &InstallRollbackInput {
                previous_settings: &current_settings,
                previous_install_state: previous_install_state.as_deref(),
                previous_favicon: previous_favicon.as_deref(),
            },
        )
        .await;
        state.runtime_settings.invalidate_cache().await;

        return match rollback_result {
            Ok(()) => {
                if let Err(runtime_rollback_error) = state
                    .storage_manager
                    .apply_runtime_settings(current_settings.clone())
                    .await
                {
                    Err(AppError::Internal(anyhow::anyhow!(
                        "安装配置应用失败，数据库已回滚，但运行态回滚失败: apply={}, runtime_rollback={}",
                        apply_error,
                        runtime_rollback_error
                    )))
                } else {
                    Err(apply_error)
                }
            }
            Err(rollback_error) => Err(AppError::Internal(anyhow::anyhow!(
                "安装配置应用失败，且数据库回滚失败: apply={}, rollback={}",
                apply_error,
                rollback_error
            ))),
        };
    }

    state.runtime_settings.invalidate_cache().await;
    let settings = validated_settings;
    let user_response = crate::models::UserResponse::from(user);

    let (access_token, refresh_token) = issue_session_tokens(
        &state,
        user_response.id,
        &user_response.email,
        &user_response.role,
    )
    .await?;

    let mut headers = HeaderMap::new();
    append_session_cookies(
        &mut headers,
        &state.config.cookie,
        &access_token,
        state.auth.access_token_ttl_seconds(),
        &refresh_token,
        state.auth.session_ttl_seconds(),
    )?;

    log_audit_db(
        &state.database,
        Some(user_response.id),
        "system.install_completed",
        "system",
        Some(user_response.id),
        None,
        Some(serde_json::json!({
            "admin_email": user_response.email,
            "site_name": settings.site_name,
            "storage_backend": settings.storage_backend.as_str(),
            "mail_enabled": settings.mail_enabled,
            "favicon_configured": favicon_data_url.is_some(),
        })),
    )
    .await;

    Ok((
        headers,
        Json(InstallBootstrapResponse {
            user: SharedUserResponse {
                email: user_response.email,
                role: user_response.role,
                created_at: user_response.created_at,
            },
            favicon_configured: favicon_data_url.is_some(),
            config: runtime_settings_to_admin_config(
                &settings,
                state.storage_manager.restart_required(&settings),
            ),
        }),
    ))
}

fn validate_favicon_data_url(value: Option<String>) -> Result<Option<String>, AppError> {
    let Some(value) = value.map(|value| value.trim().to_string()) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }

    let Some((mime_prefix, encoded)) = value.split_once(";base64,") else {
        return Err(AppError::ValidationError(
            "网站图标必须使用 data URL(base64) 格式上传".to_string(),
        ));
    };
    let Some(mime) = mime_prefix.strip_prefix("data:") else {
        return Err(AppError::ValidationError("网站图标格式无效".to_string()));
    };

    if !matches!(
        mime,
        "image/x-icon"
            | "image/vnd.microsoft.icon"
            | "image/png"
            | "image/svg+xml"
            | "image/jpeg"
            | "image/webp"
    ) {
        return Err(AppError::ValidationError(
            "网站图标仅支持 ico/png/svg/jpeg/webp".to_string(),
        ));
    }

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| AppError::ValidationError("网站图标内容无法解析".to_string()))?;
    if bytes.is_empty() {
        return Err(AppError::ValidationError("网站图标不能为空".to_string()));
    }
    if bytes.len() > MAX_FAVICON_BYTES {
        return Err(AppError::ValidationError(format!(
            "网站图标不能超过 {} KB",
            MAX_FAVICON_BYTES / 1024
        )));
    }

    Ok(Some(format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )))
}

fn public_install_status_config(
    settings: &RuntimeSettings,
    installed: bool,
    restart_required: bool,
) -> AdminSettingsConfig {
    let mut config = runtime_settings_to_admin_config(
        settings,
        if installed { false } else { restart_required },
    );

    // `install/status` is intentionally public because the login shell and first-run flow
    // need basic bootstrap state. Do not expose runtime connection details or secret-bearing
    // config here.
    config.mail_smtp_user = None;
    config.mail_smtp_password_set = false;
    config.local_storage_path.clear();
    config.mail_smtp_host.clear();
    config.mail_smtp_port = 0;
    config.mail_from_email.clear();
    config.mail_from_name.clear();
    config.mail_link_base_url.clear();
    config.settings_version.clear();

    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::StorageBackendKind;
    use crate::runtime_settings::StorageBackend;

    fn sample_runtime_settings() -> RuntimeSettings {
        RuntimeSettings {
            site_name: "Avenrixa".to_string(),
            storage_backend: StorageBackend::Local,
            local_storage_path: "/data/images".to_string(),
            mail_enabled: true,
            mail_smtp_host: "smtp.example.com".to_string(),
            mail_smtp_port: 587,
            mail_smtp_user: Some("mailer".to_string()),
            mail_smtp_password: Some("secret".to_string()),
            mail_from_email: "noreply@example.com".to_string(),
            mail_from_name: "Avenrixa".to_string(),
            mail_link_base_url: "https://img.example.com/reset".to_string(),
        }
    }

    #[test]
    fn public_install_status_config_redacts_credentials_before_install() {
        let config = public_install_status_config(&sample_runtime_settings(), false, false);

        assert!(config.local_storage_path.is_empty());
        assert!(config.mail_smtp_host.is_empty());
        assert_eq!(config.mail_smtp_port, 0);
        assert!(config.mail_from_email.is_empty());
        assert!(config.mail_enabled);
        assert_eq!(config.mail_smtp_user, None);
        assert!(!config.mail_smtp_password_set);
        assert!(config.settings_version.is_empty());
    }

    #[test]
    fn public_install_status_config_redacts_runtime_details_after_install() {
        let config = public_install_status_config(&sample_runtime_settings(), true, true);

        assert_eq!(config.site_name, "Avenrixa");
        assert_eq!(config.storage_backend, StorageBackendKind::Local);
        assert!(config.mail_enabled);
        assert!(config.local_storage_path.is_empty());
        assert!(config.mail_smtp_host.is_empty());
        assert_eq!(config.mail_smtp_port, 0);
        assert!(config.mail_from_email.is_empty());
        assert!(!config.restart_required);
        assert!(config.settings_version.is_empty());
    }
}
