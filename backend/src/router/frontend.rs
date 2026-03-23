use crate::config::Config;
use crate::db::{AppState, SITE_FAVICON_DATA_URL_SETTING_KEY, get_setting_value};
use axum::{
    Router,
    body::Body,
    extract::{OriginalUri, State},
    http::StatusCode,
    http::{HeaderValue, header},
    response::Response,
    routing,
};
use base64::Engine;
use std::{
    path::{Component, Path, PathBuf},
    sync::Arc,
};

const DEFAULT_FAVICON_SVG: &str = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>🖼️</text></svg>";

pub(super) fn create_frontend_routes(state: AppState, config: &Config) -> Router {
    let frontend_files = FrontendFiles::new(&config.server.frontend_dir);
    let spa_fallback = spa_fallback(frontend_files);

    Router::new()
        .route("/favicon.ico", routing::get(serve_favicon))
        .fallback_service(spa_fallback)
        .with_state(state)
}

pub(super) fn create_bootstrap_frontend_routes<S>(config: &Config) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let frontend_files = FrontendFiles::new(&config.server.frontend_dir);
    let spa_fallback = spa_fallback(frontend_files);

    Router::new()
        .route("/favicon.ico", routing::get(default_favicon))
        .fallback_service(spa_fallback)
}

#[derive(Clone)]
struct FrontendFiles {
    root: Arc<PathBuf>,
    index_html: Arc<PathBuf>,
}

impl FrontendFiles {
    fn new(frontend_dir: &str) -> Self {
        let root = Arc::new(PathBuf::from(frontend_dir));
        let index_html = Arc::new(root.join("index.html"));
        Self { root, index_html }
    }
}

fn spa_fallback<S>(frontend_files: FrontendFiles) -> axum::routing::MethodRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    let get_files = frontend_files.clone();
    let head_files = frontend_files;

    routing::get(move |uri: OriginalUri| {
        let get_files = get_files.clone();
        async move { serve_frontend_path(uri, get_files).await }
    })
    .head(move |uri: OriginalUri| {
        let head_files = head_files.clone();
        async move { serve_frontend_path(uri, head_files).await }
    })
}

async fn serve_frontend_path(
    OriginalUri(uri): OriginalUri,
    frontend_files: FrontendFiles,
) -> Result<Response, StatusCode> {
    let request_path = uri.path().trim_start_matches('/');

    if is_reserved_backend_path(request_path) {
        return Err(StatusCode::NOT_FOUND);
    }

    if request_path.is_empty() {
        return serve_static_file(frontend_files.index_html.as_ref(), true).await;
    }

    let Some(candidate) = resolve_frontend_candidate(frontend_files.root.as_ref(), request_path)
    else {
        return Err(StatusCode::NOT_FOUND);
    };

    if let Ok(metadata) = tokio::fs::metadata(&candidate).await {
        if metadata.is_file() {
            return serve_static_file(&candidate, candidate == *frontend_files.index_html).await;
        }

        if metadata.is_dir() {
            let directory_index = candidate.join("index.html");
            if tokio::fs::try_exists(&directory_index)
                .await
                .unwrap_or(false)
            {
                return serve_static_file(&directory_index, true).await;
            }
        }
    }

    if is_spa_route(request_path) {
        return serve_static_file(frontend_files.index_html.as_ref(), true).await;
    }

    Err(StatusCode::NOT_FOUND)
}

fn resolve_frontend_candidate(root: &Path, request_path: &str) -> Option<PathBuf> {
    let mut candidate = root.to_path_buf();
    for component in Path::new(request_path).components() {
        match component {
            Component::Normal(segment) => candidate.push(segment),
            Component::CurDir => {}
            Component::RootDir | Component::ParentDir | Component::Prefix(_) => return None,
        }
    }
    Some(candidate)
}

fn is_spa_route(request_path: &str) -> bool {
    Path::new(request_path).extension().is_none()
}

fn is_reserved_backend_path(request_path: &str) -> bool {
    matches!(request_path, "api" | "health" | "images" | "thumbnails")
        || request_path.starts_with("api/")
        || request_path.starts_with("images/")
        || request_path.starts_with("thumbnails/")
}

async fn serve_static_file(path: &Path, disable_cache: bool) -> Result<Response, StatusCode> {
    let body = tokio::fs::read(path)
        .await
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    let content_type = mime_guess::from_path(path)
        .first_or_octet_stream()
        .essence_str()
        .to_string();

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type);

    if disable_cache {
        builder = builder.header(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, no-cache, must-revalidate, max-age=0"),
        );
    }

    builder
        .body(Body::from(body))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn serve_favicon(State(state): State<AppState>) -> Result<Response, StatusCode> {
    let configured = get_setting_value(&state.database, SITE_FAVICON_DATA_URL_SETTING_KEY)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(data_url) = configured
        && let Some((content_type, bytes)) = decode_data_url(&data_url)
    {
        return Ok(favicon_response(bytes, &content_type));
    }

    Ok(favicon_response(
        DEFAULT_FAVICON_SVG.as_bytes().to_vec(),
        "image/svg+xml",
    ))
}

fn decode_data_url(data_url: &str) -> Option<(String, Vec<u8>)> {
    let trimmed = data_url.trim();
    let (mime_prefix, encoded) = trimmed.split_once(";base64,")?;
    let mime = mime_prefix.strip_prefix("data:")?.trim();
    if mime.is_empty() {
        return None;
    }

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .ok()?;
    if bytes.is_empty() {
        return None;
    }

    Some((mime.to_string(), bytes))
}

fn favicon_response(bytes: Vec<u8>, content_type: &str) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, no-cache, must-revalidate, max-age=0"),
        )
        .body(Body::from(bytes))
        .unwrap()
}

async fn default_favicon() -> Response {
    favicon_response(DEFAULT_FAVICON_SVG.as_bytes().to_vec(), "image/svg+xml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    async fn test_router() -> (tempfile::TempDir, Router) {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        std::fs::write(
            temp_dir.path().join("index.html"),
            "<!DOCTYPE html><html><body>spa shell</body></html>",
        )
        .expect("index.html should be written");
        std::fs::write(
            temp_dir.path().join("frontend.js"),
            "console.log('asset served');",
        )
        .expect("frontend.js should be written");

        let mut config = Config::default();
        config.server.frontend_dir = temp_dir.path().to_string_lossy().into_owned();

        (temp_dir, create_bootstrap_frontend_routes::<()>(&config))
    }

    #[tokio::test]
    async fn spa_route_falls_back_to_index_html() {
        let (_temp_dir, app) = test_router().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/settings")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL),
            Some(&HeaderValue::from_static(
                "no-store, no-cache, must-revalidate, max-age=0"
            ))
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        assert_eq!(
            std::str::from_utf8(&body).expect("body should be utf-8"),
            "<!DOCTYPE html><html><body>spa shell</body></html>"
        );
    }

    #[tokio::test]
    async fn missing_static_asset_stays_not_found() {
        let (_temp_dir, app) = test_router().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/missing-asset.js")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn unknown_api_route_does_not_fall_back_to_index_html() {
        let (_temp_dir, app) = test_router().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/backup-restore/status")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn existing_static_asset_is_served_directly() {
        let (_temp_dir, app) = test_router().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/frontend.js")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.contains("javascript"))
        );
        assert!(response.headers().get(header::CACHE_CONTROL).is_none());

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        assert_eq!(
            std::str::from_utf8(&body).expect("body should be utf-8"),
            "console.log('asset served');"
        );
    }
}
