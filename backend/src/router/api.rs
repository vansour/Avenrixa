use crate::config::Config;
use crate::db::AppState;
use crate::handlers::admin;
use crate::routes::{api, media};
use axum::{Router, http::StatusCode, routing};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

pub(super) fn create_root_routes(state: AppState) -> Router {
    Router::new()
        .route("/health", routing::get(admin::health_check))
        .route("/images/{filename}", routing::get(media::serve_image))
        .route(
            "/thumbnails/{image_key}",
            routing::get(media::serve_thumbnail),
        )
        .with_state(state)
}

pub(super) fn create_api_v1_router(state: AppState, config: &Config) -> Router {
    let mut governor_conf = GovernorConfigBuilder::default();
    governor_conf
        .per_second(config.server.rate_limit_per_second as u64)
        .burst_size(config.server.rate_limit_burst);
    let governor_conf = governor_conf
        .finish()
        .expect("Invalid rate limit configuration");

    let api_v1_routes = api::create_boot_public_routes()
        .merge(api::create_throttled_api_routes().layer(GovernorLayer::new(governor_conf)))
        .fallback(api_not_found)
        .with_state(state);

    Router::new().nest("/api/v1", api_v1_routes)
}

async fn api_not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Method, Request},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn unknown_post_api_route_returns_not_found() {
        let app = Router::new().nest("/api/v1", Router::new().fallback(api_not_found));

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/backups/test.sql/restore/precheck")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
