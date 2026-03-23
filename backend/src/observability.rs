use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::models::{
    BackgroundTaskMetrics, RuntimeBacklogMetrics, RuntimeObservabilitySnapshot,
    RuntimeOperationMetrics,
};

const MAX_ERROR_LENGTH: usize = 240;

#[derive(Debug, Default)]
pub struct RuntimeObservability {
    inner: Mutex<RuntimeObservabilityState>,
}

#[derive(Debug, Default)]
struct RuntimeObservabilityState {
    audit_writes: OperationState,
    auth_refresh: OperationState,
    image_processing: OperationState,
    backups: OperationState,
    background_tasks: BTreeMap<String, BackgroundTaskState>,
}

#[derive(Debug, Default)]
struct OperationState {
    total_successes: u64,
    total_failures: u64,
    last_duration_ms: Option<u64>,
    total_duration_ms: u128,
    duration_samples: u64,
    max_duration_ms: Option<u64>,
    last_success_at: Option<DateTime<Utc>>,
    last_failure_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
}

#[derive(Debug, Default)]
struct BackgroundTaskState {
    total_runs: u64,
    total_failures: u64,
    consecutive_failures: u64,
    last_duration_ms: Option<u64>,
    last_success_at: Option<DateTime<Utc>>,
    last_failure_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
}

impl RuntimeObservability {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_background_task(&self, task_name: &'static str) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        inner
            .background_tasks
            .entry(task_name.to_string())
            .or_default();
    }

    pub fn record_background_task_success(&self, task_name: &str, duration: Duration) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        inner
            .background_tasks
            .entry(task_name.to_string())
            .or_default()
            .record_success(duration);
    }

    pub fn record_background_task_failure(
        &self,
        task_name: &str,
        duration: Duration,
        error: impl ToString,
    ) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        inner
            .background_tasks
            .entry(task_name.to_string())
            .or_default()
            .record_failure(duration, error);
    }

    pub fn record_audit_success(&self, duration: Duration) {
        self.record_operation_success(OperationKind::AuditWrites, duration);
    }

    pub fn record_audit_failure(&self, duration: Duration, error: impl ToString) {
        self.record_operation_failure(OperationKind::AuditWrites, duration, error);
    }

    pub fn record_auth_refresh_success(&self, duration: Duration) {
        self.record_operation_success(OperationKind::AuthRefresh, duration);
    }

    pub fn record_auth_refresh_failure(&self, duration: Duration, error: impl ToString) {
        self.record_operation_failure(OperationKind::AuthRefresh, duration, error);
    }

    pub fn record_image_processing_success(&self, duration: Duration) {
        self.record_operation_success(OperationKind::ImageProcessing, duration);
    }

    pub fn record_image_processing_failure(&self, duration: Duration, error: impl ToString) {
        self.record_operation_failure(OperationKind::ImageProcessing, duration, error);
    }

    pub fn record_backup_success(&self, duration: Duration) {
        self.record_operation_success(OperationKind::Backups, duration);
    }

    pub fn record_backup_failure(&self, duration: Duration, error: impl ToString) {
        self.record_operation_failure(OperationKind::Backups, duration, error);
    }

    pub fn snapshot(&self, backlog: RuntimeBacklogMetrics) -> RuntimeObservabilitySnapshot {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        RuntimeObservabilitySnapshot {
            audit_writes: inner.audit_writes.snapshot(),
            auth_refresh: inner.auth_refresh.snapshot(),
            image_processing: inner.image_processing.snapshot(),
            backups: inner.backups.snapshot(),
            background_tasks: inner
                .background_tasks
                .iter()
                .map(|(task_name, state)| state.snapshot(task_name.clone()))
                .collect(),
            backlog,
        }
    }

    fn record_operation_success(&self, operation: OperationKind, duration: Duration) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        inner.operation_mut(operation).record_success(duration);
    }

    fn record_operation_failure(
        &self,
        operation: OperationKind,
        duration: Duration,
        error: impl ToString,
    ) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        inner
            .operation_mut(operation)
            .record_failure(duration, error);
    }
}

#[derive(Debug, Clone, Copy)]
enum OperationKind {
    AuditWrites,
    AuthRefresh,
    ImageProcessing,
    Backups,
}

impl RuntimeObservabilityState {
    fn operation_mut(&mut self, operation: OperationKind) -> &mut OperationState {
        match operation {
            OperationKind::AuditWrites => &mut self.audit_writes,
            OperationKind::AuthRefresh => &mut self.auth_refresh,
            OperationKind::ImageProcessing => &mut self.image_processing,
            OperationKind::Backups => &mut self.backups,
        }
    }
}

impl OperationState {
    fn record_success(&mut self, duration: Duration) {
        self.total_successes += 1;
        self.record_duration(duration);
        self.last_success_at = Some(Utc::now());
    }

    fn record_failure(&mut self, duration: Duration, error: impl ToString) {
        self.total_failures += 1;
        self.record_duration(duration);
        self.last_failure_at = Some(Utc::now());
        self.last_error = Some(trim_message(error.to_string()));
    }

    fn record_duration(&mut self, duration: Duration) {
        let duration_ms = duration_ms(duration);
        self.last_duration_ms = Some(duration_ms);
        self.total_duration_ms = self.total_duration_ms.saturating_add(duration_ms as u128);
        self.duration_samples = self.duration_samples.saturating_add(1);
        self.max_duration_ms = Some(self.max_duration_ms.unwrap_or(0).max(duration_ms));
    }

    fn snapshot(&self) -> RuntimeOperationMetrics {
        RuntimeOperationMetrics {
            total_successes: self.total_successes,
            total_failures: self.total_failures,
            last_duration_ms: self.last_duration_ms,
            average_duration_ms: if self.duration_samples == 0 {
                None
            } else {
                Some((self.total_duration_ms / self.duration_samples as u128) as u64)
            },
            max_duration_ms: self.max_duration_ms,
            last_success_at: self.last_success_at,
            last_failure_at: self.last_failure_at,
            last_error: self.last_error.clone(),
        }
    }
}

impl BackgroundTaskState {
    fn record_success(&mut self, duration: Duration) {
        self.total_runs += 1;
        self.consecutive_failures = 0;
        self.last_duration_ms = Some(duration_ms(duration));
        self.last_success_at = Some(Utc::now());
    }

    fn record_failure(&mut self, duration: Duration, error: impl ToString) {
        self.total_runs += 1;
        self.total_failures += 1;
        self.consecutive_failures += 1;
        self.last_duration_ms = Some(duration_ms(duration));
        self.last_failure_at = Some(Utc::now());
        self.last_error = Some(trim_message(error.to_string()));
    }

    fn snapshot(&self, task_name: String) -> BackgroundTaskMetrics {
        BackgroundTaskMetrics {
            task_name,
            total_runs: self.total_runs,
            total_failures: self.total_failures,
            consecutive_failures: self.consecutive_failures,
            last_duration_ms: self.last_duration_ms,
            last_success_at: self.last_success_at,
            last_failure_at: self.last_failure_at,
            last_error: self.last_error.clone(),
        }
    }
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}

fn trim_message(message: String) -> String {
    let trimmed = message.trim();
    if trimmed.chars().count() <= MAX_ERROR_LENGTH {
        return trimmed.to_string();
    }

    let mut truncated = trimmed.chars().take(MAX_ERROR_LENGTH).collect::<String>();
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_keeps_background_task_order_stable() {
        let observability = RuntimeObservability::new();
        observability.register_background_task("cleanup.storage_jobs");
        observability.register_background_task("cleanup.expired_images");

        let snapshot = observability.snapshot(RuntimeBacklogMetrics {
            storage_cleanup_pending: 0,
            storage_cleanup_retrying: 0,
        });

        let names = snapshot
            .background_tasks
            .into_iter()
            .map(|task| task.task_name)
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec!["cleanup.expired_images", "cleanup.storage_jobs"]
        );
    }

    #[test]
    fn operation_metrics_capture_success_and_failure() {
        let observability = RuntimeObservability::new();

        observability.record_audit_success(Duration::from_millis(20));
        observability.record_audit_failure(Duration::from_millis(40), "db timeout");

        let snapshot = observability.snapshot(RuntimeBacklogMetrics {
            storage_cleanup_pending: 3,
            storage_cleanup_retrying: 1,
        });

        assert_eq!(snapshot.audit_writes.total_successes, 1);
        assert_eq!(snapshot.audit_writes.total_failures, 1);
        assert_eq!(snapshot.audit_writes.average_duration_ms, Some(30));
        assert_eq!(snapshot.backlog.storage_cleanup_pending, 3);
        assert_eq!(
            snapshot.audit_writes.last_error.as_deref(),
            Some("db timeout")
        );
    }
}
