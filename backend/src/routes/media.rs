use crate::db::AppState;
use crate::middleware::AuthUser;
use axum::http::{StatusCode, header};
use axum::{
    body::Body,
    extract::{Path, State},
    response::Response,
};
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

fn private_media_response(content_type: &str, etag: &str, body: Body) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, PRIVATE_MEDIA_CACHE_CONTROL)
        .header(header::VARY, PRIVATE_MEDIA_VARY)
        .header(header::ETAG, etag)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
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

    let thumbnail = state
        .image_domain_service
        .load_thumbnail_media(image_key, auth_user.id)
        .await
        .map_err(|error| {
            error!(
                "Failed to load thumbnail media for {}: {}",
                image_key, error
            );
            match error {
                crate::error::AppError::ImageNotFound => StatusCode::NOT_FOUND,
                crate::error::AppError::Forbidden => StatusCode::FORBIDDEN,
                crate::error::AppError::IoError(_) => StatusCode::NOT_FOUND,
                crate::error::AppError::InvalidImageFormat
                | crate::error::AppError::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(private_media_response(
        &thumbnail.content_type,
        &thumbnail.etag,
        Body::from(thumbnail.data),
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

    let image = state
        .image_domain_service
        .load_image_media(&filename, auth_user.id)
        .await
        .map_err(|error| {
            error!("Failed to load image media for {}: {}", filename, error);
            match error {
                crate::error::AppError::ImageNotFound => StatusCode::NOT_FOUND,
                crate::error::AppError::Forbidden => StatusCode::FORBIDDEN,
                crate::error::AppError::IoError(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(private_media_response(
        &image.content_type,
        &image.etag,
        Body::from(image.data),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_media_response_uses_non_shared_cache_headers() {
        let response =
            private_media_response("image/png", "\"etag-value\"", Body::from(Vec::<u8>::new()));

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
        assert_eq!(
            response.headers().get(header::ETAG),
            Some(&axum::http::HeaderValue::from_static("\"etag-value\""))
        );
    }
}
