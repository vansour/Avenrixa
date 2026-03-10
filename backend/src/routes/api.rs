use crate::db::AppState;
use crate::handlers::{admin, auth, bootstrap, images, install};
use axum::{Router, routing};

pub fn create_api_routes() -> Router<AppState> {
    Router::new()
        .merge(public_routes())
        .merge(protected_routes())
        .merge(admin_routes())
}

fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/health", routing::get(admin::health_check))
        .route(
            "/bootstrap/status",
            routing::get(bootstrap::get_runtime_bootstrap_status),
        )
        .route(
            "/bootstrap/database-config",
            routing::put(bootstrap::reject_runtime_database_config_update),
        )
        .route("/install/status", routing::get(install::get_install_status))
        .route(
            "/install/bootstrap",
            routing::post(install::bootstrap_installation),
        )
        .route("/auth/login", routing::post(auth::login))
        .route("/auth/register", routing::post(auth::register))
        .route(
            "/auth/register/verify",
            routing::post(auth::verify_registration_email),
        )
        .route("/auth/refresh", routing::post(auth::refresh_session))
        .route(
            "/auth/password-reset/request",
            routing::post(auth::request_password_reset),
        )
        .route(
            "/auth/password-reset/confirm",
            routing::post(auth::confirm_password_reset),
        )
}

fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/cleanup", routing::post(admin::cleanup_deleted_files))
        .route(
            "/cleanup/expired",
            routing::post(admin::cleanup_expired_images),
        )
        .route("/backup", routing::post(admin::backup_database))
        .route("/backups", routing::get(admin::get_backups))
        .route(
            "/backups/{filename}",
            routing::get(admin::download_backup).delete(admin::delete_backup),
        )
        .route(
            "/backups/{filename}/restore/precheck",
            routing::post(admin::precheck_restore),
        )
        .route(
            "/backups/{filename}/restore",
            routing::post(admin::schedule_restore),
        )
        .route(
            "/backup-restore/status",
            routing::get(admin::get_restore_status),
        )
        .route("/users", routing::get(admin::get_users))
        .route("/users/{id}", routing::put(admin::update_user_role))
        .route("/audit-logs", routing::get(admin::get_audit_logs))
        .route("/stats", routing::get(admin::get_system_stats))
        .route(
            "/settings/config",
            routing::get(admin::get_admin_settings_config),
        )
        .route(
            "/settings/config",
            routing::put(admin::update_admin_settings_config),
        )
        .route("/settings", routing::get(admin::get_settings_admin))
        .route("/settings/{key}", routing::put(admin::update_setting))
}

fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/upload", routing::post(images::upload_image))
        .route("/images", routing::get(images::get_images))
        .route("/images", routing::delete(images::delete_images))
        .route("/images/{image_key}", routing::get(images::get_image))
        .route("/images/{image_key}", routing::put(images::update_image))
        .route(
            "/images/{image_key}/expiry",
            routing::put(images::set_expiry),
        )
        .route("/images/deleted", routing::get(images::get_deleted_images))
        .route("/images/restore", routing::post(images::restore_images))
        .route("/auth/me", routing::get(auth::get_current_user))
        .route(
            "/auth/change-password",
            routing::post(auth::change_password),
        )
        .route("/auth/logout", routing::post(auth::logout))
}
