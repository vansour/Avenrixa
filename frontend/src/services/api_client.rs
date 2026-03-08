use crate::store::auth::AuthStore;
use crate::types::errors::{AppError, Result};
use crate::utils::cookie::{build_auth_cookie, extract_auth_token_from_set_cookie};
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

/// API 客户端
#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
    auth_store: AuthStore,
}

impl ApiClient {
    pub fn new(base_url: String, auth_store: AuthStore) -> Self {
        Self {
            client: Client::new(),
            base_url,
            auth_store,
        }
    }

    /// 构建完整 URL
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// 获取认证 token
    fn get_token(&self) -> Option<String> {
        self.auth_store.token()
    }

    /// 构建带认证的请求头
    fn build_headers(&self) -> header::HeaderMap {
        let mut headers = header::HeaderMap::new();

        // 从 Store 获取 token 并添加到请求
        if let Some(token) = self.get_token() {
            let cookie_value = build_auth_cookie(&token, 7);
            let header_value = header::HeaderValue::from_str(&cookie_value).unwrap();
            headers.insert(header::COOKIE, header_value);
        }

        headers
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

    /// 处理响应，提取新的 token
    fn handle_response_headers(&self, headers: &header::HeaderMap) {
        if let Some(set_cookie) = headers.get(header::SET_COOKIE)
            && let Some(token) =
                extract_auth_token_from_set_cookie(set_cookie.to_str().unwrap_or(""))
        {
            self.auth_store.login_from_token(&token);
        }
    }

    /// 处理 HTTP 响应的通用方法
    async fn handle_response(&self, response: Response) -> Result<Response> {
        self.handle_response_headers(response.headers());

        if !response.status().is_success() {
            let status_code = response.status().as_u16();
            return Err(AppError::Request(format!("HTTP错误: {}", status_code)));
        }

        Ok(response)
    }

    /// 发送 GET 请求
    pub async fn get(&self, path: &str) -> Result<String> {
        let url = self.url(path);
        let headers = self.build_headers();

        let response = self.client.get(url).headers(headers).send().await?;
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
            .client
            .post(url)
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
            .client
            .put(url)
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

        let response = self.client.delete(url).headers(headers).send().await?;
        let _ = self.handle_response(response).await?;

        Ok(())
    }

    /// 发送带认证的 GET 请求（与 get 方法相同，保留向后兼容）
    pub async fn get_with_auth(&self, path: &str) -> Result<String> {
        self.get(path).await
    }
}
