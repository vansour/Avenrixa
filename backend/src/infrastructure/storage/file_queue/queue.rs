use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant};
use tracing::info;

use super::{
    CANCEL_TTL_SECONDS, FileSaveResult, FileSaveTask, RESULT_POLL_INTERVAL, cancel_key, result_key,
};

async fn wait_for_result(
    mut redis: ConnectionManager,
    result_key: &str,
    timeout: Duration,
) -> Result<FileSaveResult, String> {
    let started = Instant::now();

    loop {
        let status: Option<String> = redis
            .get(result_key)
            .await
            .map_err(|error| format!("读取任务结果失败: {}", error))?;

        if let Some(status) = status {
            let _: Result<i32, _> = redis.del(result_key).await;
            if let Some(result) = FileSaveResult::from_status(&status) {
                return Ok(result);
            }
            return Err(format!("未知任务结果状态: {}", status));
        }

        if started.elapsed() >= timeout {
            return Err("等待文件落盘超时".to_string());
        }

        tokio::time::sleep(RESULT_POLL_INTERVAL).await;
    }
}

/// 文件保存队列
pub enum FileSaveQueue {
    Real {
        redis: ConnectionManager,
        queue_key: String,
        _handle: JoinHandle<()>,
    },
    Mock,
}

impl FileSaveQueue {
    /// 创建新的文件保存队列
    pub fn new(
        redis: ConnectionManager,
        worker_redis: ConnectionManager,
        queue_key: String,
    ) -> Self {
        let queue_key_clone = queue_key.clone();
        let handle = tokio::spawn(async move {
            Self::process_queue(worker_redis, queue_key_clone).await;
        });

        Self::Real {
            redis,
            queue_key,
            _handle: handle,
        }
    }

    /// 创建 Mock 文件保存队列 (用于测试)
    pub fn new_mock() -> Self {
        Self::Mock
    }

    /// 返回任务结果键（如果为真实 Redis 队列）
    pub fn result_key_for_task(&self, task_id: &str) -> Option<String> {
        match self {
            Self::Real { queue_key, .. } => Some(result_key(queue_key, task_id)),
            Self::Mock => None,
        }
    }

    /// 标记任务取消（用于上传等待超时后的补偿）
    pub async fn cancel_task(&self, task_id: &str) -> Result<(), String> {
        match self {
            Self::Real {
                redis, queue_key, ..
            } => {
                let mut conn = redis.clone();
                let key = cancel_key(queue_key, task_id);
                let _: () = conn
                    .set_ex(key, "1", CANCEL_TTL_SECONDS)
                    .await
                    .map_err(|error| error.to_string())?;
                Ok(())
            }
            Self::Mock => Ok(()),
        }
    }

    /// 查询任务结果（不删除结果键）
    pub async fn get_task_result(&self, task_id: &str) -> Result<Option<FileSaveResult>, String> {
        match self {
            Self::Real {
                redis, queue_key, ..
            } => {
                let mut conn = redis.clone();
                let key = result_key(queue_key, task_id);
                let status: Option<String> =
                    conn.get(key).await.map_err(|error| error.to_string())?;
                Ok(status.and_then(|status| FileSaveResult::from_status(&status)))
            }
            Self::Mock => Ok(Some(FileSaveResult::Success)),
        }
    }

    /// 提交文件保存任务并存入 Redis
    pub async fn submit(&self, task: FileSaveTask) -> Result<(), String> {
        match self {
            Self::Real {
                redis, queue_key, ..
            } => {
                let mut conn = redis.clone();
                let payload =
                    serde_json::to_string(&task.normalized()).map_err(|error| error.to_string())?;

                let _: () = conn
                    .lpush(queue_key, payload)
                    .await
                    .map_err(|error| error.to_string())?;
                Ok(())
            }
            Self::Mock => {
                info!("Mock file save task submitted: {}", task.image_id);
                Ok(())
            }
        }
    }

    /// 提交任务并等待处理结果（用于保证文件落盘后再写数据库）
    pub async fn submit_and_wait(
        &self,
        task: FileSaveTask,
        timeout: Duration,
    ) -> Result<FileSaveResult, String> {
        match self {
            Self::Real {
                redis, queue_key, ..
            } => {
                let mut conn = redis.clone();
                let mut task = task.normalized();
                let task_id = task.task_id.clone();
                let key = task
                    .result_key
                    .clone()
                    .unwrap_or_else(|| result_key(queue_key, &task_id));
                task.result_key = Some(key.clone());

                let payload = serde_json::to_string(&task).map_err(|error| error.to_string())?;
                let _: () = conn
                    .lpush(queue_key, payload)
                    .await
                    .map_err(|error| error.to_string())?;

                wait_for_result(conn, &key, timeout).await
            }
            Self::Mock => Ok(FileSaveResult::Success),
        }
    }
}
