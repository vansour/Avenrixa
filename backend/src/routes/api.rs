use crate::db::AppState;
use axum::{Router, routing};

pub fn create_routes() -> Router<AppState> {
    Router::new()
        .merge(public_routes())
        .merge(protected_routes())
        .merge(admin_routes())
}

fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/health", routing::get(crate::admin_handlers::health_check))
        .route("/auth/login", routing::post(crate::auth_handlers::login))
        .route(
            "/auth/refresh",
            routing::post(crate::auth_handlers::refresh_session),
        )
        .route(
            "/auth/password-reset/request",
            routing::post(crate::auth_handlers::request_password_reset),
        )
        .route(
            "/auth/password-reset/confirm",
            routing::post(crate::auth_handlers::confirm_password_reset),
        )
}

fn admin_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/cleanup",
            routing::post(crate::admin_handlers::cleanup_deleted_files),
        )
        .route(
            "/cleanup/expired",
            routing::post(crate::admin_handlers::cleanup_expired_images),
        )
        .route(
            "/backup",
            routing::post(crate::admin_handlers::backup_database),
        )
        .route("/users", routing::get(crate::admin_handlers::get_users))
        .route(
            "/users/{id}",
            routing::put(crate::admin_handlers::update_user_role),
        )
        .route(
            "/audit-logs",
            routing::get(crate::admin_handlers::get_audit_logs),
        )
        .route(
            "/stats",
            routing::get(crate::admin_handlers::get_system_stats),
        )
        .route(
            "/settings/config",
            routing::get(crate::admin_handlers::get_admin_settings_config),
        )
        .route(
            "/settings/config",
            routing::put(crate::admin_handlers::update_admin_settings_config),
        )
        .route(
            "/settings",
            routing::get(crate::admin_handlers::get_settings_admin),
        )
        .route(
            "/settings/{key}",
            routing::put(crate::admin_handlers::update_setting),
        )
}

fn protected_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/upload",
            routing::post(crate::image_handlers::upload_image),
        )
        .route("/images", routing::get(crate::image_handlers::get_images))
        .route(
            "/images",
            routing::delete(crate::image_handlers::delete_images),
        )
        .route(
            "/images/{image_key}",
            routing::get(crate::image_handlers::get_image),
        )
        .route(
            "/images/{image_key}",
            routing::put(crate::image_handlers::update_image),
        )
        .route(
            "/images/{image_key}/expiry",
            routing::put(crate::image_handlers::set_expiry),
        )
        .route(
            "/images/deleted",
            routing::get(crate::image_handlers::get_deleted_images),
        )
        .route(
            "/images/restore",
            routing::post(crate::image_handlers::restore_images),
        )
        .route(
            "/auth/me",
            routing::get(crate::auth_handlers::get_current_user),
        )
        .route(
            "/auth/change-password",
            routing::post(crate::auth_handlers::change_password),
        )
        .route("/auth/logout", routing::post(crate::auth_handlers::logout))
}
