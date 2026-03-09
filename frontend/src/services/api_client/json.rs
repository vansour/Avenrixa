use crate::types::errors::Result;

use super::{ApiClient, deserialize_json, serialize_json};

impl ApiClient {
    pub async fn get_json<T: for<'de> serde::Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let response_text = self.get(path).await?;
        deserialize_json(&response_text)
    }

    pub async fn post_json<T: serde::Serialize>(&self, path: &str, body: &T) -> Result<String> {
        let json_body = serialize_json(body)?;
        self.post(path, json_body).await
    }

    pub async fn post_json_no_response<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<()> {
        let _ = self.post_json(path, body).await?;
        Ok(())
    }

    pub async fn post_json_response<T: serde::Serialize, R: for<'de> serde::Deserialize<'de>>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R> {
        let json_body = serialize_json(body)?;
        let response_text = self.post(path, json_body).await?;
        deserialize_json(&response_text)
    }

    pub async fn put_json<T: serde::Serialize>(&self, path: &str, body: &T) -> Result<String> {
        let json_body = serialize_json(body)?;
        self.put(path, json_body).await
    }

    pub async fn put_json_no_response<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<()> {
        let _ = self.put_json(path, body).await?;
        Ok(())
    }

    pub async fn put_json_response<T: serde::Serialize, R: for<'de> serde::Deserialize<'de>>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R> {
        let json_body = serialize_json(body)?;
        let response_text = self.put(path, json_body).await?;
        deserialize_json(&response_text)
    }

    pub async fn delete_json<T: serde::Serialize>(&self, path: &str, body: &T) -> Result<()> {
        let url = self.url(path);
        let headers = self.build_json_headers();
        let json_body = serialize_json(body)?;

        let send_request = || {
            self.with_credentials(self.client.delete(url.clone()))
                .headers(headers.clone())
                .body(json_body.clone())
                .send()
        };

        let mut response = send_request().await?;
        if self.should_try_refresh(path, response.status()) && self.try_refresh_session().await? {
            response = send_request().await?;
        }
        let _ = self.handle_response(response).await?;

        Ok(())
    }
}
