use redis::{AsyncCommands, RedisError, aio::ConnectionManager};
use tracing::{error, warn};

use super::super::{FileSaveQueue, FileSaveResult, FileSaveTask, RESULT_TTL_SECONDS, cancel_key};

impl FileSaveQueue {
    pub(super) async fn recover_inflight_tasks(
        redis: &mut ConnectionManager,
        queue_key: &str,
        processing_key: &str,
    ) -> Result<(), RedisError> {
        let inflight: Vec<String> = redis.lrange(processing_key, 0, -1).await?;
        if inflight.is_empty() {
            return Ok(());
        }

        warn!("检测到 {} 个处理中任务，将其恢复回主队列", inflight.len());

        for payload in inflight {
            let _: () = redis.lpush(queue_key, payload).await?;
        }
        let _: () = redis.del(processing_key).await?;
        Ok(())
    }

    pub(super) async fn is_task_cancelled(
        redis: &mut ConnectionManager,
        cancel_key: &str,
    ) -> Result<bool, RedisError> {
        redis.exists(cancel_key).await
    }

    pub(super) async fn ack_task_success(
        redis: &mut ConnectionManager,
        processing_key: &str,
        payload: &str,
        task: &FileSaveTask,
        cancel_key: &str,
    ) -> Result<(), RedisError> {
        if let Some(result_key) = &task.result_key {
            let _: () = redis
                .set_ex(
                    result_key,
                    FileSaveResult::Success.as_str(),
                    RESULT_TTL_SECONDS,
                )
                .await?;
        }
        let _: Result<i32, _> = redis.del(cancel_key).await;
        let _: i32 = redis.lrem(processing_key, 1, payload).await?;
        Ok(())
    }

    pub(super) async fn ack_task_cancelled(
        redis: &mut ConnectionManager,
        processing_key: &str,
        payload: &str,
        task: &FileSaveTask,
        cancel_key: &str,
    ) -> Result<(), RedisError> {
        if let Some(result_key) = &task.result_key {
            let _: () = redis
                .set_ex(
                    result_key,
                    FileSaveResult::Cancelled.as_str(),
                    RESULT_TTL_SECONDS,
                )
                .await?;
        }
        let _: Result<i32, _> = redis.del(cancel_key).await;
        let _: i32 = redis.lrem(processing_key, 1, payload).await?;
        Ok(())
    }

    pub(super) async fn handle_task_failure(
        redis: &mut ConnectionManager,
        queue_key: &str,
        processing_key: &str,
        dead_letter_key: &str,
        mut task: FileSaveTask,
        payload: String,
        result: FileSaveResult,
    ) -> Result<(), RedisError> {
        if task.attempts + 1 < task.max_retries {
            task.attempts += 1;
            let retry_payload = serde_json::to_string(&task).unwrap_or(payload.clone());
            let _: () = redis.lpush(queue_key, retry_payload).await?;
            let _: i32 = redis.lrem(processing_key, 1, &payload).await?;
            warn!(
                "文件保存任务重试: image_id={}, attempt={}/{}",
                task.image_id, task.attempts, task.max_retries
            );
            return Ok(());
        }

        if let Some(result_key) = &task.result_key {
            let _: () = redis
                .set_ex(result_key, result.as_str(), RESULT_TTL_SECONDS)
                .await?;
        }

        let _: () = redis.lpush(dead_letter_key, payload.clone()).await?;
        let _: i32 = redis.lrem(processing_key, 1, &payload).await?;
        let _: Result<i32, _> = redis.del(cancel_key(queue_key, &task.task_id)).await;
        error!(
            "文件保存任务达到最大重试次数，已移入死信队列: image_id={}",
            task.image_id
        );
        Ok(())
    }

    pub(super) async fn move_to_dead_letter(
        redis: &mut ConnectionManager,
        processing_key: &str,
        dead_letter_key: &str,
        payload: &str,
    ) -> Result<(), RedisError> {
        let _: () = redis.lpush(dead_letter_key, payload).await?;
        let _: i32 = redis.lrem(processing_key, 1, payload).await?;
        Ok(())
    }
}
