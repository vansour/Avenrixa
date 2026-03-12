use crate::db::AppState;
use crate::db::DatabasePool;
use crate::middleware::AuthUser;
use axum::http::{StatusCode, header};
use axum::{
    body::Body,
    extract::{Path, State},
    response::Response,
};
use image::ImageFormat;
use tracing::error;

const PRIVATE_MEDIA_CACHE_CONTROL: &str = "private, no-store, max-age=0";
const PRIVATE_MEDIA_VARY: &str = "Cookie, Authorization";

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
        DatabasePool::MySql(pool) => {
            sqlx::query_scalar::<_, String>(
                "SELECT filename
                 FROM images
                 WHERE hash = ?
                   AND user_id = ?
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
        DatabasePool::MySql(pool) => {
            sqlx::query_scalar::<_, i32>(
                "SELECT 1
                 FROM images
                 WHERE filename = ?
                   AND user_id = ?
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

fn private_media_response(content_type: &str, body: Body) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, PRIVATE_MEDIA_CACHE_CONTROL)
        .header(header::VARY, PRIVATE_MEDIA_VARY)
        .body(body)
        .unwrap()
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

    Ok(private_media_response(
        "image/webp",
        Body::from(cursor.into_inner()),
    ))
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

    Ok(private_media_response(mime.as_ref(), Body::from(data)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_media_response_uses_non_shared_cache_headers() {
        let response = private_media_response("image/png", Body::from(Vec::<u8>::new()));

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE),
            Some(&axum::http::HeaderValue::from_static("image/png"))
        );
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL),
            Some(&axum::http::HeaderValue::from_static(
                PRIVATE_MEDIA_CACHE_CONTROL
            ))
        );
        assert_eq!(
            response.headers().get(header::VARY),
            Some(&axum::http::HeaderValue::from_static(PRIVATE_MEDIA_VARY))
        );
    }
}
