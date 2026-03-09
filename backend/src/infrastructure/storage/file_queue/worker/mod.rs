mod redis_ops;

use redis::{RedisError, aio::ConnectionManager};
use tracing::{error, info, warn};

use super::{FileSaveQueue, FileSaveResult, FileSaveTask, cancel_key};

#[derive(Default)]
struct WorkerStats {
    task_count: usize,
    success_count: usize,
    failure_count: usize,
    cancelled_count: usize,
}

impl WorkerStats {
    fn log_if_needed(&self) {
        if self.task_count > 0 && self.task_count.is_multiple_of(100) {
            info!(
                "文件保存任务统计: 总数={}, 成功={}, 失败={}, 取消={}",
                self.task_count, self.success_count, self.failure_count, self.cancelled_count
            );
        }
    }
}

fn is_timeout_error(error: &RedisError) -> bool {
    error.to_string().to_lowercase().contains("timed out")
}

impl FileSaveQueue {
    /// 处理任务队列
    pub(super) async fn process_queue(mut redis: ConnectionManager, queue_key: String) {
        let processing_key = format!("{}:processing", queue_key);
        let dead_letter_key = format!("{}:dead_letter", queue_key);

        info!(
            "Redis 文件保存任务队列已启动: queue={}, processing={}, dlq={}",
            queue_key, processing_key, dead_letter_key
        );

        if let Err(error) =
            Self::recover_inflight_tasks(&mut redis, &queue_key, &processing_key).await
        {
            warn!("恢复处理中任务失败: {}", error);
        }

        let mut stats = WorkerStats::default();

        loop {
            let result: Result<Option<String>, _> = redis::cmd("BRPOPLPUSH")
                .arg(&queue_key)
                .arg(&processing_key)
                .arg(5)
                .query_async(&mut redis)
                .await;

            match result {
                Ok(Some(payload)) => {
                    stats.task_count += 1;
                    Self::handle_payload(
                        &mut redis,
                        &queue_key,
                        &processing_key,
                        &dead_letter_key,
                        payload,
                        &mut stats,
                    )
                    .await;
                }
                Ok(None) => continue,
                Err(error) => {
                    if is_timeout_error(&error) {
                        continue;
                    }
                    error!("Redis BRPOP error: {}", error);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }

            stats.log_if_needed();
        }
    }

    async fn handle_payload(
        redis: &mut ConnectionManager,
        queue_key: &str,
        processing_key: &str,
        dead_letter_key: &str,
        payload: String,
        stats: &mut WorkerStats,
    ) {
        let task = match serde_json::from_str::<FileSaveTask>(&payload) {
            Ok(task) => task,
            Err(_) => {
                error!("Failed to deserialize task payload");
                stats.failure_count += 1;
                if let Err(error) =
                    Self::move_to_dead_letter(redis, processing_key, dead_letter_key, &payload)
                        .await
                {
                    error!("无法移动损坏任务到死信队列: {}", error);
                }
                return;
            }
        };

        let task_cancel_key = cancel_key(queue_key, &task.task_id);
        if Self::is_task_cancelled(redis, &task_cancel_key)
            .await
            .unwrap_or(false)
        {
            stats.cancelled_count += 1;
            Self::handle_cancelled_task(redis, processing_key, &payload, &task, &task_cancel_key)
                .await;
            return;
        }

        let result = Self::process_task(task.clone()).await;
        match result {
            FileSaveResult::Success => {
                if Self::is_task_cancelled(redis, &task_cancel_key)
                    .await
                    .unwrap_or(false)
                {
                    stats.cancelled_count += 1;
                    Self::handle_cancelled_task(
                        redis,
                        processing_key,
                        &payload,
                        &task,
                        &task_cancel_key,
                    )
                    .await;
                } else {
                    stats.success_count += 1;
                    if let Err(error) = Self::ack_task_success(
                        redis,
                        processing_key,
                        &payload,
                        &task,
                        &task_cancel_key,
                    )
                    .await
                    {
                        error!("任务确认成功失败: {}", error);
                    }
                }
            }
            other => {
                if Self::is_task_cancelled(redis, &task_cancel_key)
                    .await
                    .unwrap_or(false)
                {
                    stats.cancelled_count += 1;
                    Self::handle_cancelled_task(
                        redis,
                        processing_key,
                        &payload,
                        &task,
                        &task_cancel_key,
                    )
                    .await;
                } else {
                    stats.failure_count += 1;
                    if let Err(error) = Self::handle_task_failure(
                        redis,
                        queue_key,
                        processing_key,
                        dead_letter_key,
                        task,
                        payload,
                        other,
                    )
                    .await
                    {
                        error!("处理失败任务时出错: {}", error);
                    }
                }
            }
        }
    }

    async fn handle_cancelled_task(
        redis: &mut ConnectionManager,
        processing_key: &str,
        payload: &str,
        task: &FileSaveTask,
        cancel_key: &str,
    ) {
        if let Err(error) = Self::cleanup_task_artifacts(task).await {
            warn!("清理已取消任务文件失败 [{}]: {}", task.image_id, error);
        }
        if let Err(error) =
            Self::ack_task_cancelled(redis, processing_key, payload, task, cancel_key).await
        {
            error!("确认取消任务失败: {}", error);
        }
    }
}
