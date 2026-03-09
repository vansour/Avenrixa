use crate::db::AppState;
use axum::http::{StatusCode, header};
use axum::{
    Router,
    extract::{Path, State},
    response::Response,
    routing,
};
use image::ImageFormat;
use tracing::error;

fn is_valid_image_key(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_valid_filename(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 255
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains("..")
}

/// 动态处理缩略图请求（/thumbnails/{image_key}.webp），不落盘缩略图
pub(crate) async fn serve_thumbnail(
    Path(path_key): Path<String>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let Some(image_key) = path_key.strip_suffix(".webp") else {
        return Err(StatusCode::NOT_FOUND);
    };
    if !is_valid_image_key(image_key) {
        return Err(StatusCode::NOT_FOUND);
    }

    let filename = sqlx::query_scalar::<_, String>(
        "SELECT filename FROM images WHERE hash = $1 AND deleted_at IS NULL AND status = 'active' LIMIT 1",
    )
    .bind(image_key)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        error!("Failed to query image by hash: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    let data = state.storage_manager.read(&filename).await.map_err(|e| {
        error!("Failed to read source image for thumbnail: {}", e);
        StatusCode::NOT_FOUND
    })?;

    let image = image::load_from_memory(&data).map_err(|e| {
        error!("Failed to decode source image for thumbnail: {}", e);
        StatusCode::UNPROCESSABLE_ENTITY
    })?;
    let thumb = image.resize(
        state.config.image.thumbnail_size,
        state.config.image.thumbnail_size,
        image::imageops::FilterType::Lanczos3,
    );
    let mut cursor = std::io::Cursor::new(Vec::new());
    thumb
        .write_to(&mut cursor, ImageFormat::WebP)
        .map_err(|e| {
            error!("Failed to encode dynamic thumbnail as webp: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/webp")
        .header(header::CACHE_CONTROL, "public, max-age=86400")
        .body(axum::body::Body::from(cursor.into_inner()))
        .unwrap())
}

/// 动态处理原图请求（/images/{filename}）
pub(crate) async fn serve_image(
    Path(filename): Path<String>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    if !is_valid_filename(&filename) {
        return Err(StatusCode::NOT_FOUND);
    }

    let exists = sqlx::query_scalar::<_, i32>(
        "SELECT 1 FROM images WHERE filename = $1 AND deleted_at IS NULL AND status = 'active' LIMIT 1",
    )
    .bind(&filename)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        error!("Failed to query image by filename: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .is_some();

    if !exists {
        return Err(StatusCode::NOT_FOUND);
    }

    let data = state.storage_manager.read(&filename).await.map_err(|e| {
        error!("Failed to read source image: {}", e);
        StatusCode::NOT_FOUND
    })?;
    let mime = mime_guess::from_path(&filename).first_or_octet_stream();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime.as_ref())
        .header(header::CACHE_CONTROL, "public, max-age=86400")
        .body(axum::body::Body::from(data))
        .unwrap())
}

pub fn create_routes() -> Router<AppState> {
    // 公共路由（无需认证）
    let public_routes = Router::new()
        .route("/health", routing::get(crate::admin_handlers::health_check))
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
        );

    // 需要认证的路由（使用 AuthUser 中间件）
    let protected_routes = Router::new()
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
            "/images/{image_key}/edit",
            routing::post(crate::image_handlers::edit_image),
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
        .route("/auth/logout", routing::post(crate::auth_handlers::logout));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(admin_routes)
}
