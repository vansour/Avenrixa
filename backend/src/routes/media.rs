use crate::db::AppState;
use crate::middleware::AuthUser;
use axum::http::{HeaderMap, StatusCode, header};
use axum::{
    body::Body,
    extract::{Path, State},
    response::Response,
};
use tokio_util::io::ReaderStream;
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

#[cfg(test)]
fn private_media_response(content_type: &str, etag: &str, body: Body) -> Response {
    private_media_response_with_status(StatusCode::OK, Some(content_type), etag, None, body)
}

fn private_media_streaming_response(
    content_type: &str,
    etag: &str,
    content_length: u64,
    body: Body,
) -> Response {
    private_media_response_with_status(
        StatusCode::OK,
        Some(content_type),
        etag,
        Some(content_length),
        body,
    )
}

fn private_media_not_modified_response(etag: &str) -> Response {
    private_media_response_with_status(StatusCode::NOT_MODIFIED, None, etag, None, Body::empty())
}

fn private_media_response_with_status(
    status: StatusCode,
    content_type: Option<&str>,
    etag: &str,
    content_length: Option<u64>,
    body: Body,
) -> Response {
    let mut builder = Response::builder()
        .status(status)
        .header(header::CACHE_CONTROL, PRIVATE_MEDIA_CACHE_CONTROL)
        .header(header::VARY, PRIVATE_MEDIA_VARY)
        .header(header::ETAG, etag)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff");

    if let Some(content_type) = content_type {
        builder = builder.header(header::CONTENT_TYPE, content_type);
    }

    if let Some(content_length) = content_length {
        builder = builder.header(header::CONTENT_LENGTH, content_length.to_string());
    }

    builder.body(body).unwrap()
}

fn if_none_match_matches(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get_all(header::IF_NONE_MATCH)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .any(|candidate| candidate == "*" || candidate == etag)
}

pub(crate) async fn serve_thumbnail(
    Path(path_key): Path<String>,
    headers: HeaderMap,
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

    if if_none_match_matches(&headers, &thumbnail.etag) {
        return Ok(private_media_not_modified_response(&thumbnail.etag));
    }

    let handle = state
        .storage_manager
        .open_read(&thumbnail.file_key)
        .await
        .map_err(|error| {
            error!(
                "Failed to open thumbnail media stream for {}: {}",
                thumbnail.file_key, error
            );
            match error {
                crate::error::AppError::IoError(_) => StatusCode::NOT_FOUND,
                crate::error::AppError::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(private_media_streaming_response(
        &thumbnail.content_type,
        &thumbnail.etag,
        handle.content_length,
        Body::from_stream(ReaderStream::new(handle.file)),
    ))
}

pub(crate) async fn serve_image(
    Path(filename): Path<String>,
    headers: HeaderMap,
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

    if if_none_match_matches(&headers, &image.etag) {
        return Ok(private_media_not_modified_response(&image.etag));
    }

    let handle = state
        .storage_manager
        .open_read(&image.file_key)
        .await
        .map_err(|error| {
            error!(
                "Failed to open image media stream for {}: {}",
                filename, error
            );
            match error {
                crate::error::AppError::IoError(_) => StatusCode::NOT_FOUND,
                crate::error::AppError::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(private_media_streaming_response(
        &image.content_type,
        &image.etag,
        handle.content_length,
        Body::from_stream(ReaderStream::new(handle.file)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

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

    #[test]
    fn if_none_match_matches_exact_etag() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::IF_NONE_MATCH,
            HeaderValue::from_static("\"etag-value\""),
        );

        assert!(if_none_match_matches(&headers, "\"etag-value\""));
    }

    #[test]
    fn if_none_match_matches_wildcard() {
        let mut headers = HeaderMap::new();
        headers.insert(header::IF_NONE_MATCH, HeaderValue::from_static("*"));

        assert!(if_none_match_matches(&headers, "\"etag-value\""));
    }

    #[test]
    fn private_media_not_modified_response_uses_304_and_etag() {
        let response = private_media_not_modified_response("\"etag-value\"");

        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(
            response.headers().get(header::ETAG),
            Some(&HeaderValue::from_static("\"etag-value\""))
        );
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL),
            Some(&HeaderValue::from_static(PRIVATE_MEDIA_CACHE_CONTROL))
        );
    }
}
