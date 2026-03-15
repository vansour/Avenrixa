mod api;
mod bootstrap;
mod frontend;

use crate::bootstrap::BootstrapAppState;
use crate::config::Config;
use crate::db::AppState;
use axum::Router;

pub fn create_app_router(state: AppState, config: &Config) -> Router {
    Router::new()
        .merge(api::create_root_routes(state.clone()))
        .merge(api::create_api_v1_router(state.clone(), config))
        .merge(frontend::create_frontend_routes(state.clone(), config))
}

pub fn create_app_with_middleware(
    state: AppState,
    config: &Config,
    max_upload_size: usize,
) -> Router {
    create_app_router(state, config).layer(axum::extract::DefaultBodyLimit::max(max_upload_size))
}

pub fn create_bootstrap_app(
    state: BootstrapAppState,
    config: &Config,
    max_upload_size: usize,
) -> Router {
    bootstrap::create_bootstrap_router(state, config)
        .layer(axum::extract::DefaultBodyLimit::max(max_upload_size))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthService;
    use crate::bootstrap::build_app_state;
    use crate::db::{DatabasePool, mark_app_installed_sqlite_tx};
    use crate::domain::auth::state_repository::AuthStateRepository;
    use crate::models::{InstallStatusResponse, UserRole};
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use chrono::Utc;
    use serde_json::json;
    use sqlx::SqlitePool;
    use std::sync::OnceLock;
    use tower::ServiceExt;
    use uuid::Uuid;

    const PRIVATE_MEDIA_CACHE_CONTROL: &str = "private, no-store, max-age=0";
    const PRIVATE_MEDIA_VARY: &str = "Cookie, Authorization";

    fn ensure_test_env() {
        static INIT: OnceLock<()> = OnceLock::new();
        INIT.get_or_init(|| {
            #[allow(unused_unsafe)]
            unsafe {
                std::env::set_var(
                    "JWT_SECRET",
                    "test-jwt-secret-with-sufficient-length-1234567890",
                );
                std::env::set_var("ENV", "test");
            }
        });
    }

    fn sample_png_bytes() -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]));
        let mut cursor = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .expect("png encoding should succeed");
        cursor.into_inner()
    }

    struct TestApp {
        _temp_dir: tempfile::TempDir,
        state: crate::db::AppState,
    }

    impl TestApp {
        async fn new(configure: impl FnOnce(&mut Config)) -> Self {
            ensure_test_env();

            let temp_dir = tempfile::tempdir().expect("temp dir should be created");
            let frontend_dir = temp_dir.path().join("frontend");
            std::fs::create_dir_all(&frontend_dir).expect("frontend dir should be created");
            std::fs::write(
                frontend_dir.join("index.html"),
                "<!DOCTYPE html><html><body>test shell</body></html>",
            )
            .expect("index.html should be written");

            let mut config = Config::default();
            config.database.kind = crate::config::DatabaseKind::Sqlite;
            config.database.max_connections = 1;
            config.database.url = temp_dir
                .path()
                .join("app.db")
                .to_string_lossy()
                .into_owned();
            config.storage.path = temp_dir
                .path()
                .join("storage")
                .to_string_lossy()
                .into_owned();
            config.server.frontend_dir = frontend_dir.to_string_lossy().into_owned();
            config.cookie.secure = false;

            configure(&mut config);

            let state = build_app_state(config.clone())
                .await
                .expect("app state should build");

            Self {
                _temp_dir: temp_dir,
                state,
            }
        }

        fn sqlite_pool(&self) -> &SqlitePool {
            match &self.state.database {
                DatabasePool::Sqlite(pool) => pool,
                DatabasePool::Postgres(_) | DatabasePool::MySql(_) => {
                    panic!("test app should use sqlite")
                }
            }
        }

        fn router(&self) -> Router {
            Router::new()
                .merge(super::api::create_root_routes(self.state.clone()))
                .nest(
                    "/api/v1",
                    crate::routes::api::create_api_routes().with_state(self.state.clone()),
                )
        }

        async fn request(&self, request: Request<Body>) -> axum::response::Response {
            self.router()
                .oneshot(request)
                .await
                .expect("request should succeed")
        }

        async fn mark_installed(&self) {
            let mut tx = self.sqlite_pool().begin().await.expect("tx should start");
            mark_app_installed_sqlite_tx(&mut tx)
                .await
                .expect("install flag should be set");
            tx.commit().await.expect("tx should commit");
        }

        async fn insert_user(&self, id: Uuid, email: &str, role: &str) {
            let password_hash =
                AuthService::hash_password("Password123!").expect("password hash should succeed");
            sqlx::query(
                "INSERT INTO users (id, email, email_verified_at, password_hash, role, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .bind(id)
            .bind(email)
            .bind(Some(Utc::now()))
            .bind(password_hash)
            .bind(role)
            .bind(Utc::now())
            .execute(self.sqlite_pool())
            .await
            .expect("user should be inserted");
        }

        async fn insert_active_image(&self, user_id: Uuid, filename: &str, hash: &str) {
            self.state
                .storage_manager
                .write(filename, &sample_png_bytes())
                .await
                .expect("image file should be written");

            sqlx::query(
                "INSERT INTO images (id, user_id, filename, thumbnail, size, hash, format, views, status, expires_at, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )
            .bind(Uuid::new_v4())
            .bind(user_id)
            .bind(filename)
            .bind(None::<String>)
            .bind(sample_png_bytes().len() as i64)
            .bind(hash)
            .bind("png")
            .bind(0_i64)
            .bind("active")
            .bind(None::<chrono::DateTime<Utc>>)
            .bind(Utc::now())
            .execute(self.sqlite_pool())
            .await
            .expect("image record should be inserted");
        }

        async fn auth_cookie(
            &self,
            user_id: Uuid,
            email: &str,
            role: &str,
        ) -> axum::http::HeaderValue {
            let token_version = self
                .state
                .auth_state_repository
                .get_user_token_version(user_id)
                .await
                .expect("token version should load")
                .unwrap_or(0);
            let session_epoch = self
                .state
                .auth_state_repository
                .get_session_epoch()
                .await
                .expect("session epoch should load");
            let token = self
                .state
                .auth
                .generate_token(
                    user_id,
                    email,
                    &UserRole::parse(role),
                    token_version,
                    session_epoch,
                )
                .expect("token should be generated");
            axum::http::HeaderValue::from_str(&format!("auth_token={token}"))
                .expect("cookie header should be valid")
        }
    }

    #[tokio::test]
    async fn media_route_sets_private_cache_headers() {
        let app = TestApp::new(|_| {}).await;
        app.mark_installed().await;

        let user_id = Uuid::new_v4();
        let filename = "sample.png";
        app.insert_user(user_id, "user@example.com", "user").await;
        app.insert_active_image(user_id, filename, &"a".repeat(64))
            .await;

        let response = app
            .request(
                Request::builder()
                    .uri(format!("/images/{filename}"))
                    .header(
                        header::COOKIE,
                        app.auth_cookie(user_id, "user@example.com", "user").await,
                    )
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
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

    #[tokio::test]
    async fn thumbnail_route_sets_private_cache_headers() {
        let app = TestApp::new(|_| {}).await;
        app.mark_installed().await;

        let user_id = Uuid::new_v4();
        let hash = "b".repeat(64);
        app.insert_user(user_id, "user@example.com", "user").await;
        app.insert_active_image(user_id, "thumb-source.png", &hash)
            .await;

        let response = app
            .request(
                Request::builder()
                    .uri(format!("/thumbnails/{hash}.webp"))
                    .header(
                        header::COOKIE,
                        app.auth_cookie(user_id, "user@example.com", "user").await,
                    )
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
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

    #[tokio::test]
    async fn anonymous_install_status_redacts_runtime_details_before_install() {
        let app = TestApp::new(|config| {
            config.mail.enabled = true;
            config.mail.smtp_host = "smtp.internal.example".to_string();
            config.mail.smtp_port = 2525;
            config.mail.smtp_user = Some("mailer".to_string());
            config.mail.smtp_password = Some("super-secret".to_string());
            config.mail.from_email = "ops@example.com".to_string();
            config.mail.from_name = "Ops".to_string();
            config.mail.reset_link_base_url = "https://img.example.com/reset".to_string();
        })
        .await;

        let response = app
            .request(
                Request::builder()
                    .uri("/api/v1/install/status")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let status: InstallStatusResponse =
            serde_json::from_slice(&body).expect("body should decode");

        assert!(!status.installed);
        assert!(status.config.mail_enabled);
        assert!(status.config.local_storage_path.is_empty());
        assert!(status.config.mail_smtp_host.is_empty());
        assert_eq!(status.config.mail_smtp_port, 0);
        assert!(status.config.mail_from_email.is_empty());
        assert!(status.config.mail_from_name.is_empty());
        assert!(status.config.mail_link_base_url.is_empty());
        assert_eq!(status.config.mail_smtp_user, None);
        assert!(!status.config.mail_smtp_password_set);
        assert_eq!(status.config.s3_endpoint, None);
        assert_eq!(status.config.s3_bucket, None);
        assert_eq!(status.config.s3_access_key, None);
        assert!(!status.config.s3_secret_key_set);
    }

    #[tokio::test]
    async fn anonymous_install_status_redacts_runtime_details_after_install() {
        let app = TestApp::new(|config| {
            config.mail.enabled = true;
            config.mail.smtp_host = "smtp.internal.example".to_string();
            config.mail.from_email = "ops@example.com".to_string();
        })
        .await;
        app.mark_installed().await;

        let response = app
            .request(
                Request::builder()
                    .uri("/api/v1/install/status")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let status: InstallStatusResponse =
            serde_json::from_slice(&body).expect("body should decode");

        assert!(status.installed);
        assert!(status.config.local_storage_path.is_empty());
        assert!(status.config.mail_smtp_host.is_empty());
        assert_eq!(status.config.mail_smtp_port, 0);
        assert!(status.config.mail_from_email.is_empty());
        assert_eq!(status.config.s3_endpoint, None);
        assert_eq!(status.config.s3_bucket, None);
        assert_eq!(status.config.s3_access_key, None);
    }

    #[tokio::test]
    async fn install_bootstrap_applies_local_storage_path_without_restart() {
        let app = TestApp::new(|_| {}).await;
        let selected_path = app._temp_dir.path().join("mounted-images");

        let response = app
            .request(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/install/bootstrap")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "admin_email": "admin@example.com",
                            "admin_password": "Password123!",
                            "favicon_data_url": null,
                            "config": {
                                "site_name": "Vansour Image",
                                "storage_backend": "local",
                                "local_storage_path": selected_path.to_string_lossy(),
                                "mail_enabled": false,
                                "mail_smtp_host": "",
                                "mail_smtp_port": 587,
                                "mail_smtp_user": null,
                                "mail_smtp_password": null,
                                "mail_from_email": "",
                                "mail_from_name": "",
                                "mail_link_base_url": "",
                                "s3_endpoint": null,
                                "s3_region": null,
                                "s3_bucket": null,
                                "s3_prefix": null,
                                "s3_access_key": null,
                                "s3_secret_key": null,
                                "s3_force_path_style": true
                            }
                        }))
                        .expect("request body should serialize"),
                    ))
                    .expect("request should build"),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            app.state
                .storage_manager
                .active_settings()
                .local_storage_path,
            selected_path.to_string_lossy().to_string()
        );

        let proof_file = "install-proof.png";
        app.state
            .storage_manager
            .write(proof_file, &sample_png_bytes())
            .await
            .expect("proof file should be written");

        assert!(
            tokio::fs::try_exists(selected_path.join(proof_file))
                .await
                .expect("existence check should succeed")
        );
    }

    #[tokio::test]
    async fn last_admin_demotion_is_rejected_via_route() {
        let app = TestApp::new(|_| {}).await;
        app.mark_installed().await;

        let admin_id = Uuid::new_v4();
        app.insert_user(admin_id, "admin@example.com", "admin")
            .await;

        let response = app
            .request(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/v1/users/{admin_id}"))
                    .header(
                        header::COOKIE,
                        app.auth_cookie(admin_id, "admin@example.com", "admin")
                            .await,
                    )
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"role":"user"}"#))
                    .expect("request should build"),
            )
            .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let json: serde_json::Value =
            serde_json::from_slice(&body).expect("body should decode as json");
        assert_eq!(
            json.get("code").and_then(|value| value.as_str()),
            Some("VALIDATION_ERROR")
        );
        assert_eq!(
            json.get("error").and_then(|value| value.as_str()),
            Some("系统至少需要保留一个管理员账户")
        );
    }

    #[tokio::test]
    async fn demoted_admin_session_is_invalidated_for_follow_up_requests() {
        let app = TestApp::new(|_| {}).await;
        app.mark_installed().await;

        let demoted_admin_id = Uuid::new_v4();
        let second_admin_id = Uuid::new_v4();
        app.insert_user(demoted_admin_id, "first-admin@example.com", "admin")
            .await;
        app.insert_user(second_admin_id, "second-admin@example.com", "admin")
            .await;

        let stale_cookie = app
            .auth_cookie(demoted_admin_id, "first-admin@example.com", "admin")
            .await;

        let demote_response = app
            .request(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/v1/users/{demoted_admin_id}"))
                    .header(header::COOKIE, stale_cookie.clone())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"role":"user"}"#))
                    .expect("request should build"),
            )
            .await;

        assert_eq!(demote_response.status(), StatusCode::OK);

        let follow_up_response = app
            .request(
                Request::builder()
                    .uri("/api/v1/stats")
                    .header(header::COOKIE, stale_cookie)
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await;

        assert_eq!(follow_up_response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn password_change_invalidates_stale_session_for_follow_up_requests() {
        let app = TestApp::new(|_| {}).await;
        app.mark_installed().await;

        let user_id = Uuid::new_v4();
        app.insert_user(user_id, "user@example.com", "user").await;

        let stale_cookie = app.auth_cookie(user_id, "user@example.com", "user").await;

        let change_password_response = app
            .request(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/change-password")
                    .header(header::COOKIE, stale_cookie.clone())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"current_password":"Password123!","new_password":"NewPassword123!"}"#,
                    ))
                    .expect("request should build"),
            )
            .await;

        assert_eq!(change_password_response.status(), StatusCode::OK);

        let follow_up_response = app
            .request(
                Request::builder()
                    .uri("/api/v1/auth/me")
                    .header(header::COOKIE, stale_cookie)
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await;

        assert_eq!(follow_up_response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn runtime_router_rejects_public_bootstrap_database_updates() {
        let app = TestApp::new(|_| {}).await;

        let response = app
            .request(
                Request::builder()
                    .method("PUT")
                    .uri("/api/v1/bootstrap/database-config")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"database_kind":"sqlite","database_url":"sqlite:///tmp/test.db"}"#,
                    ))
                    .expect("request should build"),
            )
            .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let json: serde_json::Value =
            serde_json::from_slice(&body).expect("body should decode as json");
        assert_eq!(
            json.get("code").and_then(|value| value.as_str()),
            Some("VALIDATION_ERROR")
        );
    }
}
