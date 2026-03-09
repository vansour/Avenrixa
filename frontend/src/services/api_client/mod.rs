mod core;
mod json;
mod multipart;

use crate::types::errors::{AppError, Result};
use reqwest::header;
use reqwest::{Client, Response};

fn serialize_json<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|error| AppError::Server(format!("JSON序列化失败: {}", error)))
}

fn deserialize_json<T: for<'de> serde::Deserialize<'de>>(json: &str) -> Result<T> {
    serde_json::from_str(json)
        .map_err(|error| AppError::Server(format!("JSON反序列化失败: {}", error)))
}

fn extract_error_message(body: &str) -> Option<String> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        for key in ["message", "error", "detail"] {
            if let Some(message) = value.get(key).and_then(|entry| entry.as_str()) {
                let message = message.trim();
                if !message.is_empty() {
                    return Some(message.to_string());
                }
            }
        }
    }

    Some(trimmed.chars().take(200).collect())
}

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder().build().unwrap_or_else(|_| Client::new());
        Self { client, base_url }
    }

    pub(super) fn build_headers(&self) -> header::HeaderMap {
        header::HeaderMap::new()
    }

    pub(super) fn build_json_headers(&self) -> header::HeaderMap {
        let mut headers = self.build_headers();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers
    }

    pub(super) async fn handle_response(&self, response: Response) -> Result<Response> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }

        let status_code = status.as_u16();
        let response_text = response.text().await.unwrap_or_default();
        let detail = extract_error_message(&response_text);

        match status_code {
            400 => Err(AppError::Validation(
                detail.unwrap_or_else(|| "请求参数错误".to_string()),
            )),
            401 => Err(AppError::Unauthorized),
            403 => Err(AppError::Forbidden),
            404 => Err(AppError::NotFound),
            429 => Err(AppError::Request(
                detail.unwrap_or_else(|| "请求过于频繁".to_string()),
            )),
            500 => Err(AppError::Server(
                detail.unwrap_or_else(|| "服务器内部错误".to_string()),
            )),
            502 => Err(AppError::Server(
                detail.unwrap_or_else(|| "网关错误".to_string()),
            )),
            503 => Err(AppError::Server(
                detail.unwrap_or_else(|| "服务不可用".to_string()),
            )),
            _ => {
                let message = detail.unwrap_or_else(|| "网络错误".to_string());
                Err(AppError::Request(format!(
                    "{} (HTTP {})",
                    message, status_code
                )))
            }
        }
    }

    pub(super) fn with_credentials(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> reqwest::RequestBuilder {
        #[cfg(target_arch = "wasm32")]
        {
            builder.fetch_credentials_include()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            builder
        }
    }
}
