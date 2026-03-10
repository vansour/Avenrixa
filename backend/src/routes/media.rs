use crate::db::AppState;
use crate::db::DatabasePool;
use crate::middleware::AuthUser;
use axum::http::{StatusCode, header};
use axum::{
    extract::{Path, State},
    response::Response,
};
use image::ImageFormat;
use tracing::error;

fn is_valid_image_key(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|character| character.is_ascii_hexdigit())
}

fn is_valid_filename(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 255
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains("..")
}

async fn find_active_filename_by_hash(
    database: &DatabasePool,
    image_key: &str,
    user_id: uuid::Uuid,
) -> Result<Option<String>, sqlx::Error> {
    match database {
        DatabasePool::Postgres(pool) => {
            sqlx::query_scalar::<_, String>(
                "SELECT filename
                 FROM images
                 WHERE hash = $1
                   AND user_id = $2
                   AND deleted_at IS NULL
                   AND status = 'active'
                 LIMIT 1",
            )
            .bind(image_key)
            .bind(user_id)
            .fetch_optional(pool)
            .await
        }
        DatabasePool::Sqlite(pool) => {
            sqlx::query_scalar::<_, String>(
                "SELECT filename
                 FROM images
                 WHERE hash = ?1
                   AND user_id = ?2
                   AND deleted_at IS NULL
                   AND status = 'active'
                 LIMIT 1",
            )
            .bind(image_key)
            .bind(user_id)
            .fetch_optional(pool)
            .await
        }
    }
}

async fn active_image_exists_by_filename(
    database: &DatabasePool,
    filename: &str,
    user_id: uuid::Uuid,
) -> Result<bool, sqlx::Error> {
    let exists = match database {
        DatabasePool::Postgres(pool) => {
            sqlx::query_scalar::<_, i32>(
                "SELECT 1
                 FROM images
                 WHERE filename = $1
                   AND user_id = $2
                   AND deleted_at IS NULL
                   AND status = 'active'
                 LIMIT 1",
            )
            .bind(filename)
            .bind(user_id)
            .fetch_optional(pool)
            .await?
        }
        DatabasePool::Sqlite(pool) => {
            sqlx::query_scalar::<_, i32>(
                "SELECT 1
                 FROM images
                 WHERE filename = ?1
                   AND user_id = ?2
                   AND deleted_at IS NULL
                   AND status = 'active'
                 LIMIT 1",
            )
            .bind(filename)
            .bind(user_id)
            .fetch_optional(pool)
            .await?
        }
    };

    Ok(exists.is_some())
}

pub(crate) async fn serve_thumbnail(
    Path(path_key): Path<String>,
    auth_user: AuthUser,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let Some(image_key) = path_key.strip_suffix(".webp") else {
        return Err(StatusCode::NOT_FOUND);
    };
    if !is_valid_image_key(image_key) {
        return Err(StatusCode::NOT_FOUND);
    }

    let filename = find_active_filename_by_hash(&state.database, image_key, auth_user.id)
        .await
        .map_err(|error| {
            error!("Failed to query image by hash: {}", error);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let data = state
        .storage_manager
        .read(&filename)
        .await
        .map_err(|error| {
            error!("Failed to read source image for thumbnail: {}", error);
            StatusCode::NOT_FOUND
        })?;

    let image = image::load_from_memory(&data).map_err(|error| {
        error!("Failed to decode source image for thumbnail: {}", error);
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
        .map_err(|error| {
            error!("Failed to encode dynamic thumbnail as webp: {}", error);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/webp")
        .header(header::CACHE_CONTROL, "public, max-age=86400")
        .body(axum::body::Body::from(cursor.into_inner()))
        .unwrap())
}

pub(crate) async fn serve_image(
    Path(filename): Path<String>,
    auth_user: AuthUser,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    if !is_valid_filename(&filename) {
        return Err(StatusCode::NOT_FOUND);
    }

    let exists = active_image_exists_by_filename(&state.database, &filename, auth_user.id)
        .await
        .map_err(|error| {
            error!("Failed to query image by filename: {}", error);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !exists {
        return Err(StatusCode::NOT_FOUND);
    }

    let data = state
        .storage_manager
        .read(&filename)
        .await
        .map_err(|error| {
            error!("Failed to read source image: {}", error);
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
