use crate::types::errors::{AppError, Result};
use reqwest::{Client, Response};
use reqwest::header;

/// JSON 序列化帮助函数
fn serialize_json<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(|e| AppError::Server(format!("JSON序列化失败: {}", e)))
}

/// JSON 反序列化帮助函数
fn deserialize_json<T: for<'de> serde::Deserialize<'de>>(json: &str) -> Result<T> {
    serde_json::from_str(json).map_err(|e| AppError::Server(format!("JSON反序列化失败: {}", e)))
}

/// 从服务端响应体提取可展示的错误信息
fn extract_error_message(body: &str) -> Option<String> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        for key in ["message", "error", "detail"] {
            if let Some(msg) = value.get(key).and_then(|v| v.as_str()) {
                let msg = msg.trim();
                if !msg.is_empty() {
                    return Some(msg.to_string());
                }
            }
        }
    }

    Some(trimmed.chars().take(200).collect())
}

/// API 客户端
#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client, base_url }
    }

    /// 构建完整 URL
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
                    .and_then(|w| w.location().origin().ok())
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

        // 非绝对 URL（例如 "api" 或 "/api"）按同源路径前缀处理。
        let base_path = if base.starts_with('/') {
            base.to_string()
        } else {
            format!("/{}", base)
        };

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(origin) = web_sys::window()
                .and_then(|w| w.location().origin().ok())
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

    /// 构建请求头
    fn build_headers(&self) -> header::HeaderMap {
        header::HeaderMap::new()
    }

    /// 构建带 JSON Content-Type 的请求头
    fn build_json_headers(&self) -> header::HeaderMap {
        let mut headers = self.build_headers();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers
    }

    /// 处理 HTTP 响应的通用方法
    async fn handle_response(&self, response: Response) -> Result<Response> {
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

    /// 为请求启用浏览器 Cookie 凭据策略
    fn with_credentials(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        #[cfg(target_arch = "wasm32")]
        {
            builder.fetch_credentials_include()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            builder
        }
    }

    /// 发送 GET 请求
    pub async fn get(&self, path: &str) -> Result<String> {
        let url = self.url(path);
        let headers = self.build_headers();

        let response = self
            .with_credentials(self.client.get(url))
            .headers(headers)
            .send()
            .await?;
        let response = self.handle_response(response).await?;

        response.text().await.map_err(AppError::from_reqwest)
    }

    /// 发送 GET 请求并反序列化响应
    pub async fn get_json<T: for<'de> serde::Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let response_text = self.get(path).await?;
        deserialize_json(&response_text)
    }

    /// 发送 POST 请求（带 JSON body）
    pub async fn post(&self, path: &str, body: String) -> Result<String> {
        let url = self.url(path);
        let headers = self.build_json_headers();

        let response = self
            .with_credentials(self.client.post(url))
            .headers(headers)
            .body(body)
            .send()
            .await?;
        let response = self.handle_response(response).await?;

        response.text().await.map_err(AppError::from_reqwest)
    }

    /// 发送 JSON POST 请求（自动序列化）
    pub async fn post_json<T: serde::Serialize>(&self, path: &str, body: &T) -> Result<String> {
        let json_body = serialize_json(body)?;
        self.post(path, json_body).await
    }

    /// 发送 JSON POST 请求（不要求响应体）
    pub async fn post_json_no_response<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<()> {
        let _ = self.post_json(path, body).await?;
        Ok(())
    }

    /// 发送 JSON POST 请求并反序列化响应
    pub async fn post_json_response<T: serde::Serialize, R: for<'de> serde::Deserialize<'de>>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R> {
        let json_body = serialize_json(body)?;
        let response_text = self.post(path, json_body).await?;
        deserialize_json(&response_text)
    }

    /// 发送 PUT 请求（带 JSON body）
    pub async fn put(&self, path: &str, body: String) -> Result<String> {
        let url = self.url(path);
        let headers = self.build_json_headers();

        let response = self
            .with_credentials(self.client.put(url))
            .headers(headers)
            .body(body)
            .send()
            .await?;
        let response = self.handle_response(response).await?;

        response.text().await.map_err(AppError::from_reqwest)
    }

    /// 发送 JSON PUT 请求（自动序列化）
    pub async fn put_json<T: serde::Serialize>(&self, path: &str, body: &T) -> Result<String> {
        let json_body = serialize_json(body)?;
        self.put(path, json_body).await
    }

    /// 发送 JSON PUT 请求（不要求响应体）
    pub async fn put_json_no_response<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<()> {
        let _ = self.put_json(path, body).await?;
        Ok(())
    }

    /// 发送 JSON PUT 请求并反序列化响应
    pub async fn put_json_response<T: serde::Serialize, R: for<'de> serde::Deserialize<'de>>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R> {
        let json_body = serialize_json(body)?;
        let response_text = self.put(path, json_body).await?;
        deserialize_json(&response_text)
    }

    /// 发送 DELETE 请求
    pub async fn delete(&self, path: &str) -> Result<()> {
        let url = self.url(path);
        let headers = self.build_headers();

        let response = self
            .with_credentials(self.client.delete(url))
            .headers(headers)
            .send()
            .await?;
        let _ = self.handle_response(response).await?;

        Ok(())
    }

    /// 发送带 JSON body 的 DELETE 请求
    pub async fn delete_json<T: serde::Serialize>(&self, path: &str, body: &T) -> Result<()> {
        let url = self.url(path);
        let headers = self.build_json_headers();
        let json_body = serialize_json(body)?;

        let response = self
            .with_credentials(self.client.delete(url))
            .headers(headers)
            .body(json_body)
            .send()
            .await?;
        let _ = self.handle_response(response).await?;

        Ok(())
    }

    /// 发送带认证的 GET 请求（与 get 方法相同，保留向后兼容）
    pub async fn get_with_auth(&self, path: &str) -> Result<String> {
        self.get(path).await
    }
}
