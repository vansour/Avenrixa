use crate::types::errors::{AppError, Result};
use reqwest::multipart::{Form, Part};

use super::{ApiClient, deserialize_json};

impl ApiClient {
    pub async fn post_multipart_file<R: for<'de> serde::Deserialize<'de>>(
        &self,
        path: &str,
        field_name: &str,
        filename: String,
        content_type: Option<String>,
        data: Vec<u8>,
    ) -> Result<R> {
        let url = self.url(path);

        let build_form = || {
            let mut part = Part::bytes(data.clone()).file_name(filename.clone());
            if let Some(content_type) = content_type.clone()
                && !content_type.trim().is_empty()
            {
                part = part.mime_str(&content_type).map_err(|error| {
                    AppError::Request(format!("无效文件 MIME 类型 '{}': {}", content_type, error))
                })?;
            }
            Ok::<Form, AppError>(Form::new().part(field_name.to_string(), part))
        };

        let send_request = || async {
            let form = build_form()?;
            self.with_credentials(self.client.post(url.clone()))
                .multipart(form)
                .send()
                .await
                .map_err(AppError::from_reqwest)
        };

        let mut response = send_request().await?;
        if self.should_try_refresh(path, response.status()) && self.try_refresh_session().await? {
            response = send_request().await?;
        }
        let response = self.handle_response(response).await?;
        let text = response.text().await.map_err(AppError::from_reqwest)?;

        deserialize_json(&text)
    }
}
