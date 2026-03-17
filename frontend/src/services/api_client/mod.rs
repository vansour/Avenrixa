mod core;
mod json;
mod multipart;

use crate::types::errors::{AppError, Result};
use futures::future::{LocalBoxFuture, Shared};
use reqwest::header;
use reqwest::{Client, Response};
use std::rc::Rc;
use std::sync::Mutex;

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
    refresh_coordinator: Rc<RefreshCoordinator>,
}

impl ApiClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder().build().unwrap_or_else(|_| Client::new());
        Self {
            client,
            base_url,
            refresh_coordinator: Rc::new(RefreshCoordinator::default()),
        }
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

type SharedRefreshFuture = Shared<LocalBoxFuture<'static, Result<bool>>>;

#[derive(Clone)]
struct RefreshFlight {
    id: u64,
    future: SharedRefreshFuture,
}

#[derive(Default)]
struct RefreshCoordinatorState {
    next_id: u64,
    in_flight: Option<RefreshFlight>,
}

#[derive(Default)]
struct RefreshCoordinator {
    state: Mutex<RefreshCoordinatorState>,
}

impl RefreshCoordinator {
    fn shared_refresh<F>(&self, build: F) -> (u64, SharedRefreshFuture)
    where
        F: FnOnce() -> SharedRefreshFuture,
    {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(flight) = guard.in_flight.as_ref() {
            return (flight.id, flight.future.clone());
        }

        guard.next_id = guard.next_id.wrapping_add(1);
        let id = guard.next_id;
        let future = build();
        guard.in_flight = Some(RefreshFlight {
            id,
            future: future.clone(),
        });
        (id, future)
    }

    fn finish(&self, id: u64) {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard
            .in_flight
            .as_ref()
            .is_some_and(|flight| flight.id == id)
        {
            guard.in_flight = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::FutureExt;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[tokio::test]
    async fn refresh_coordinator_reuses_single_in_flight_future() {
        let coordinator = RefreshCoordinator::default();
        let executions = Arc::new(AtomicUsize::new(0));

        let first_runs = executions.clone();
        let (first_id, first) = coordinator.shared_refresh(move || {
            async move {
                first_runs.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(20)).await;
                Ok(true)
            }
            .boxed_local()
            .shared()
        });

        let second_runs = executions.clone();
        let (second_id, second) = coordinator.shared_refresh(move || {
            async move {
                second_runs.fetch_add(1, Ordering::SeqCst);
                Ok(false)
            }
            .boxed_local()
            .shared()
        });

        assert_eq!(first_id, second_id);

        let (first_result, second_result) = tokio::join!(first, second);

        assert!(first_result.expect("first refresh should succeed"));
        assert!(second_result.expect("second refresh should succeed"));
        assert_eq!(executions.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn refresh_coordinator_keeps_newer_future_when_old_waiter_finishes_late() {
        let coordinator = RefreshCoordinator::default();

        let (old_id, old_future) =
            coordinator.shared_refresh(|| async { Ok(true) }.boxed_local().shared());
        assert!(old_future.await.expect("old refresh should succeed"));
        coordinator.finish(old_id);

        let (new_id, _new_future) =
            coordinator.shared_refresh(|| async { Ok(false) }.boxed_local().shared());
        assert_ne!(old_id, new_id);

        coordinator.finish(old_id);
        let (joined_id, _joined_future) =
            coordinator.shared_refresh(|| async { Ok(true) }.boxed_local().shared());

        assert_eq!(joined_id, new_id);
    }
}
