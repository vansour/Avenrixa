use axum::{Router, routing};
use crate::db::AppState;

pub fn create_routes() -> Router<AppState> {
    // 公共路由（无需认证）
    let public_routes = Router::new()
        .route("/health", routing::get(crate::admin_handlers::health_check))
        .route("/api/auth/register", routing::post(crate::auth_handlers::register))
        .route("/api/auth/login", routing::post(crate::auth_handlers::login))
        .route("/api/auth/forgot-password", routing::post(crate::auth_handlers::forgot_password))
        .route("/api/auth/reset-password", routing::post(crate::auth_handlers::reset_password));

    // 需要认证的 API 路由
    let protected_routes = Router::new()
        .route("/api/upload", routing::post(crate::image_handlers::upload_image))
        .route("/api/images", routing::get(crate::image_handlers::get_images))
        .route("/api/images", routing::delete(crate::image_handlers::delete_images))
        .route("/api/images/{id}", routing::get(crate::image_handlers::get_image))
        .route("/api/images/{id}/edit", routing::post(crate::image_handlers::edit_image))
        .route("/api/images/{id}", routing::put(crate::image_handlers::update_image))
        .route("/api/images/{id}/rename", routing::put(crate::image_handlers::rename_image))
        .route("/api/images/{id}/expiry", routing::put(crate::image_handlers::set_expiry))
        .route("/api/images/{id}/duplicate", routing::post(crate::image_handlers::duplicate_image))
        .route("/api/images/deleted", routing::get(crate::image_handlers::get_deleted_images))
        .route("/api/images/restore", routing::post(crate::image_handlers::restore_images))
        .route("/api/categories", routing::get(crate::category_handlers::get_categories))
        .route("/api/categories", routing::post(crate::category_handlers::create_category))
        .route("/api/categories/{id}", routing::delete(crate::category_handlers::delete_category))
        .route("/api/auth/me", routing::get(crate::auth_handlers::get_current_user))
        .route("/api/auth/change-password", routing::post(crate::auth_handlers::change_password))
        .route("/api/auth/logout", routing::post(crate::auth_handlers::logout))
        .route("/api/settings", routing::get(crate::admin_handlers::get_settings_public));

    // 管理员路由
    let admin_routes = Router::new()
        .route("/api/cleanup", routing::post(crate::admin_handlers::cleanup_deleted_files))
        .route("/api/cleanup/expired", routing::post(crate::admin_handlers::cleanup_expired_images))
        .route("/api/backup", routing::post(crate::admin_handlers::backup_database))
        .route("/api/approve", routing::post(crate::admin_handlers::approve_images))
        .route("/admin/users", routing::get(crate::admin_handlers::get_users))
        .route("/admin/users/{id}", routing::put(crate::admin_handlers::update_user_role))
        .route("/admin/audit-logs", routing::get(crate::admin_handlers::get_audit_logs))
        .route("/admin/stats", routing::get(crate::admin_handlers::get_system_stats))
        .route("/admin/settings", routing::get(crate::admin_handlers::get_settings))
        .route("/api/settings/{key}", routing::put(crate::admin_handlers::update_setting));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(admin_routes)
}
