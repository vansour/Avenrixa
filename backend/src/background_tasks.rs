use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::Utc;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{error, info};

use crate::db::{ADMIN_USER_ID, AppState};
use crate::domain::auth::state_repository::AuthStateRepository;
use crate::observability::RuntimeObservability;
use crate::storage_backend::process_pending_storage_cleanup_jobs;

const STORAGE_CLEANUP_BATCH_SIZE: usize = 64;

pub struct BackgroundTaskManager {
    tasks: Vec<ManagedIntervalTask>,
}

struct ManagedIntervalTask {
    shutdown_tx: watch::Sender<bool>,
    join_handle: JoinHandle<()>,
}

impl Drop for ManagedIntervalTask {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.send(true);
        let _ = &self.join_handle;
    }
}

impl BackgroundTaskManager {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn spawn_cleanup_tasks(state: &AppState) -> Self {
        let mut manager = Self::new();
        let config = &state.config;
        if !config.cleanup.enabled {
            info!("Background cleanup tasks are disabled by configuration");
            return manager;
        }

        let expiry_check_interval =
            Duration::from_secs(config.cleanup.expiry_check_interval_seconds);
        let storage_cleanup_interval =
            Duration::from_secs(config.cleanup.expiry_check_interval_seconds.clamp(5, 30));

        let admin_domain_service = state.admin_domain_service.clone();
        let installed_state_for_expiry = state.database.clone();
        manager.spawn_interval_task(
            "cleanup.expired_images",
            expiry_check_interval,
            state.observability.clone(),
            move || {
                let admin_domain_service = admin_domain_service.clone();
                let installed_state_for_expiry = installed_state_for_expiry.clone();
                async move {
                    if !crate::db::is_app_installed(&installed_state_for_expiry).await? {
                        return Ok(());
                    }

                    admin_domain_service
                        .cleanup_expired_images(ADMIN_USER_ID, "system")
                        .await?;
                    Ok(())
                }
            },
        );

        let auth_state_repository = state.auth_state_repository.clone();
        let installed_state_for_auth_cleanup = state.database.clone();
        manager.spawn_interval_task(
            "cleanup.revoked_tokens",
            expiry_check_interval,
            state.observability.clone(),
            move || {
                let auth_state_repository = auth_state_repository.clone();
                let installed_state_for_auth_cleanup = installed_state_for_auth_cleanup.clone();
                async move {
                    if !crate::db::is_app_installed(&installed_state_for_auth_cleanup).await? {
                        return Ok(());
                    }

                    auth_state_repository
                        .purge_expired_revoked_tokens(Utc::now())
                        .await?;
                    Ok(())
                }
            },
        );

        let storage_cleanup_database = state.database.clone();
        let installed_state_for_storage_cleanup = state.database.clone();
        manager.spawn_interval_task(
            "cleanup.storage_jobs",
            storage_cleanup_interval,
            state.observability.clone(),
            move || {
                let storage_cleanup_database = storage_cleanup_database.clone();
                let installed_state_for_storage_cleanup =
                    installed_state_for_storage_cleanup.clone();
                async move {
                    if !crate::db::is_app_installed(&installed_state_for_storage_cleanup).await? {
                        return Ok(());
                    }

                    process_pending_storage_cleanup_jobs(
                        &storage_cleanup_database,
                        STORAGE_CLEANUP_BATCH_SIZE,
                    )
                    .await?;
                    Ok(())
                }
            },
        );

        manager
    }

    fn spawn_interval_task<F, Fut>(
        &mut self,
        task_name: &'static str,
        interval: Duration,
        observability: Arc<RuntimeObservability>,
        task: F,
    ) where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let total_errors = Arc::new(AtomicU64::new(0));
        observability.register_background_task(task_name);
        let join_handle = tokio::spawn(run_interval_task(
            task_name,
            interval,
            shutdown_rx,
            total_errors.clone(),
            observability,
            task,
        ));

        info!(
            task = task_name,
            interval_secs = interval.as_secs(),
            "Background task started"
        );

        self.tasks.push(ManagedIntervalTask {
            shutdown_tx,
            join_handle,
        });
    }
}

impl Default for BackgroundTaskManager {
    fn default() -> Self {
        Self::new()
    }
}

async fn run_interval_task<F, Fut>(
    task_name: &'static str,
    interval: Duration,
    mut shutdown_rx: watch::Receiver<bool>,
    total_errors: Arc<AtomicU64>,
    observability: Arc<RuntimeObservability>,
    task: F,
) where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut consecutive_errors = 0_u64;

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                info!(
                    task = task_name,
                    total_errors = total_errors.load(Ordering::Relaxed),
                    "Background task stopping"
                );
                break;
            }
            _ = ticker.tick() => {
                let started_at = Instant::now();
                match task().await {
                    Ok(()) => {
                        observability.record_background_task_success(task_name, started_at.elapsed());
                        if consecutive_errors > 0 {
                            info!(
                                task = task_name,
                                elapsed_ms = started_at.elapsed().as_millis() as u64,
                                consecutive_errors,
                                total_errors = total_errors.load(Ordering::Relaxed),
                                "Background task recovered"
                            );
                            consecutive_errors = 0;
                        }
                    }
                    Err(error) => {
                        let elapsed = started_at.elapsed();
                        consecutive_errors += 1;
                        let seen_errors = total_errors.fetch_add(1, Ordering::Relaxed) + 1;
                        observability.record_background_task_failure(
                            task_name,
                            elapsed,
                            error.to_string(),
                        );
                        error!(
                            task = task_name,
                            elapsed_ms = elapsed.as_millis() as u64,
                            consecutive_errors,
                            total_errors = seen_errors,
                            error = %error,
                            "Background task tick failed"
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[tokio::test]
    async fn manager_drop_stops_interval_tasks() {
        let executions = Arc::new(AtomicUsize::new(0));
        let mut manager = BackgroundTaskManager::new();
        let executions_for_task = executions.clone();
        let observability = Arc::new(RuntimeObservability::new());

        manager.spawn_interval_task(
            "test.interval",
            Duration::from_millis(10),
            observability,
            move || {
                let executions = executions_for_task.clone();
                async move {
                    executions.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            },
        );

        tokio::time::sleep(Duration::from_millis(35)).await;
        let before_drop = executions.load(Ordering::Relaxed);
        drop(manager);
        tokio::time::sleep(Duration::from_millis(30)).await;
        let after_drop = executions.load(Ordering::Relaxed);

        assert!(
            after_drop <= before_drop + 1,
            "interval task kept running after manager drop: before={before_drop}, after={after_drop}"
        );
    }
}
