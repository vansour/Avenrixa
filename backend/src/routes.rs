use crate::db::AppState;
use axum::http::{StatusCode, header};
use axum::{
    Router,
    extract::{Path, State},
    response::Response,
    routing,
};
use tokio::fs;
use tracing::error;

/// 处理缩略图请求（自动添加 .jpg 扩展名）
async fn serve_thumbnail(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let thumbnail_path = format!("{}/{}.jpg", state.config.storage.thumbnail_path, id);
    match fs::read(&thumbnail_path).await {
        Ok(data) => Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "image/jpeg")
            .body(axum::body::Body::from(data))
            .unwrap()),
        Err(e) => {
            error!("Failed to read thumbnail: {}", e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

pub fn create_routes() -> Router<AppState> {
    // 公共路由（无需认证）
    let public_routes = Router::new()
        .route("/health", routing::get(crate::admin_handlers::health_check))
        .route("/thumbnails/{id}", routing::get(serve_thumbnail))
        .route("/auth/login", routing::post(crate::auth_handlers::login));

    // 管理员路由（使用 AdminUser 中间件）
    let admin_routes = Router::new()
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
            "/settings",
            routing::get(crate::admin_handlers::get_settings_admin),
        )
        .route(
            "/settings/{key}",
            routing::put(crate::admin_handlers::update_setting),
        );

    // 需要认证的路由（使用 AuthUser 中间件）
    let protected_routes = Router::new()
        .route(
            "/upload",
            routing::post(crate::image_handlers::upload_image),
        )
        .route("/images", routing::get(crate::image_handlers::get_images))
        .route(
            "/images/cursor",
            routing::get(crate::handlers::images_cursor::get_images),
        )
        .route(
            "/images",
            routing::delete(crate::image_handlers::delete_images),
        )
        .route(
            "/images/{id}",
            routing::get(crate::image_handlers::get_image),
        )
        .route(
            "/images/{id}/edit",
            routing::post(crate::image_handlers::edit_image),
        )
        .route(
            "/images/{id}",
            routing::put(crate::image_handlers::update_image),
        )
        .route(
            "/images/{id}/rename",
            routing::put(crate::image_handlers::rename_image),
        )
        .route(
            "/images/{id}/expiry",
            routing::put(crate::image_handlers::set_expiry),
        )
        .route(
            "/images/{id}/duplicate",
            routing::post(crate::image_handlers::duplicate_image),
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
        .route("/auth/logout", routing::post(crate::auth_handlers::logout));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(admin_routes)
}
