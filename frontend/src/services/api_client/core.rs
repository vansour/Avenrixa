use crate::types::errors::{AppError, Result};
use reqwest::StatusCode;

use super::ApiClient;

impl ApiClient {
    pub(super) fn should_try_refresh(&self, path: &str, status: StatusCode) -> bool {
        status == StatusCode::UNAUTHORIZED
            && !matches!(
                path,
                "/api/v1/auth/login"
                    | "/api/v1/auth/register"
                    | "/api/v1/auth/register/verify"
                    | "/api/v1/auth/logout"
                    | "/api/v1/auth/refresh"
                    | "/api/v1/auth/password-reset/request"
                    | "/api/v1/auth/password-reset/confirm"
            )
    }

    pub(super) async fn try_refresh_session(&self) -> Result<bool> {
        let url = self.url("/api/v1/auth/refresh");
        let response = self
            .with_credentials(self.client.post(url))
            .send()
            .await
            .map_err(AppError::from_reqwest)?;

        match response.status() {
            status if status.is_success() => Ok(true),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Ok(false),
            _ => {
                let _ = self.handle_response(response).await?;
                Ok(false)
            }
        }
    }

    pub fn url(&self, path: &str) -> String {
        let path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };

        let base = self.base_url.trim();
        if base.is_empty() || base == "/" {
            #[cfg(target_arch = "wasm32")]
            {
                if let Some(origin) = web_sys::window()
                    .and_then(|window| window.location().origin().ok())
                    .filter(|origin| !origin.is_empty() && origin != "null")
                {
                    return format!("{}{}", origin.trim_end_matches('/'), path);
                }
            }
            return path;
        }

        if base.starts_with("http://") || base.starts_with("https://") {
            return format!("{}{}", base.trim_end_matches('/'), path);
        }

        let base_path = if base.starts_with('/') {
            base.to_string()
        } else {
            format!("/{}", base)
        };

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(origin) = web_sys::window()
                .and_then(|window| window.location().origin().ok())
                .filter(|origin| !origin.is_empty() && origin != "null")
            {
                return format!(
                    "{}{}{}",
                    origin.trim_end_matches('/'),
                    base_path.trim_end_matches('/'),
                    path
                );
            }
        }

        format!("{}{}", base_path.trim_end_matches('/'), path)
    }

    pub async fn get(&self, path: &str) -> Result<String> {
        let url = self.url(path);
        let headers = self.build_headers();

        let send_request = || {
            self.with_credentials(self.client.get(url.clone()))
                .headers(headers.clone())
                .send()
        };

        let mut response = send_request().await?;
        if self.should_try_refresh(path, response.status()) && self.try_refresh_session().await? {
            response = send_request().await?;
        }
        let response = self.handle_response(response).await?;

        response.text().await.map_err(AppError::from_reqwest)
    }

    pub async fn post(&self, path: &str, body: String) -> Result<String> {
        let url = self.url(path);
        let headers = self.build_json_headers();

        let send_request = || {
            self.with_credentials(self.client.post(url.clone()))
                .headers(headers.clone())
                .body(body.clone())
                .send()
        };

        let mut response = send_request().await?;
        if self.should_try_refresh(path, response.status()) && self.try_refresh_session().await? {
            response = send_request().await?;
        }
        let response = self.handle_response(response).await?;

        response.text().await.map_err(AppError::from_reqwest)
    }

    pub async fn put(&self, path: &str, body: String) -> Result<String> {
        let url = self.url(path);
        let headers = self.build_json_headers();

        let send_request = || {
            self.with_credentials(self.client.put(url.clone()))
                .headers(headers.clone())
                .body(body.clone())
                .send()
        };

        let mut response = send_request().await?;
        if self.should_try_refresh(path, response.status()) && self.try_refresh_session().await? {
            response = send_request().await?;
        }
        let response = self.handle_response(response).await?;

        response.text().await.map_err(AppError::from_reqwest)
    }

    pub async fn delete(&self, path: &str) -> Result<()> {
        let url = self.url(path);
        let headers = self.build_headers();

        let send_request = || {
            self.with_credentials(self.client.delete(url.clone()))
                .headers(headers.clone())
                .send()
        };

        let mut response = send_request().await?;
        if self.should_try_refresh(path, response.status()) && self.try_refresh_session().await? {
            response = send_request().await?;
        }
        let _ = self.handle_response(response).await?;

        Ok(())
    }

    pub async fn get_with_auth(&self, path: &str) -> Result<String> {
        self.get(path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_try_refresh_only_for_protected_unauthorized_requests() {
        let client = ApiClient::new(String::new());

        assert!(client.should_try_refresh("/api/v1/images", StatusCode::UNAUTHORIZED));
        assert!(!client.should_try_refresh("/api/v1/images", StatusCode::FORBIDDEN));
        assert!(!client.should_try_refresh("/api/v1/auth/login", StatusCode::UNAUTHORIZED));
        assert!(!client.should_try_refresh("/api/v1/auth/refresh", StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn url_uses_root_relative_paths_when_base_is_empty() {
        let client = ApiClient::new(String::new());

        assert_eq!(client.url("api/v1/images"), "/api/v1/images");
        assert_eq!(client.url("/api/v1/images"), "/api/v1/images");
    }

    #[test]
    fn url_joins_absolute_origin_without_double_slashes() {
        let client = ApiClient::new("https://img.example.com/app/".to_string());

        assert_eq!(
            client.url("/api/v1/images"),
            "https://img.example.com/app/api/v1/images"
        );
    }

    #[test]
    fn url_normalizes_relative_base_paths() {
        let client = ApiClient::new("console".to_string());

        assert_eq!(client.url("api/v1/images"), "/console/api/v1/images");
    }
}
