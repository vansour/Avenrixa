//! 文件保存任务队列模块
//! 负责处理文件保存任务，确保文件写入完成后再返回给用户

use redis::AsyncCommands;
use redis::RedisError;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant};
use tracing::{error, info, warn};
use uuid::Uuid;

const DEFAULT_MAX_RETRIES: u8 = 3;
const RESULT_POLL_INTERVAL: Duration = Duration::from_millis(100);
const RESULT_TTL_SECONDS: u64 = 300;
const CANCEL_TTL_SECONDS: u64 = 600;

fn default_max_retries() -> u8 {
    DEFAULT_MAX_RETRIES
}

/// 文件保存任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSaveTask {
    #[serde(default)]
    pub task_id: String,
    pub image_id: String,
    pub storage_path: String,
    pub thumbnail_path: String,
    pub temp_image_path: String,
    pub thumbnail_data: Vec<u8>, // 缩略图通常很小，可以保留在内存中
    #[serde(default)]
    pub attempts: u8,
    #[serde(default = "default_max_retries")]
    pub max_retries: u8,
    #[serde(default)]
    pub result_key: Option<String>,
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

/// 文件保存结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileSaveResult {
    Success,
    ImageFailed,
    ThumbnailFailed,
    Cancelled,
}

impl FileSaveResult {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::ImageFailed => "image_failed",
            Self::ThumbnailFailed => "thumbnail_failed",
            Self::Cancelled => "cancelled",
        }
    }

    fn from_status(status: &str) -> Option<Self> {
        match status {
            "success" => Some(Self::Success),
            "image_failed" => Some(Self::ImageFailed),
            "thumbnail_failed" => Some(Self::ThumbnailFailed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

impl FileSaveQueue {
    fn result_key(queue_key: &str, task_id: &str) -> String {
        format!("{}:result:{}", queue_key, task_id)
    }

    fn cancel_key(queue_key: &str, task_id: &str) -> String {
        format!("{}:cancel:{}", queue_key, task_id)
    }

    /// 创建新的文件保存队列
    pub fn new(
        redis: ConnectionManager,
        worker_redis: ConnectionManager,
        queue_key: String,
    ) -> Self {
        let queue_key_clone = queue_key.clone();

        let _handle = tokio::spawn(async move {
            // 队列消费者使用专用连接，避免阻塞式 BRPOPLPUSH 影响业务命令（如健康检查 PING）。
            Self::process_queue(worker_redis, queue_key_clone).await;
        });

        Self::Real {
            redis,
            queue_key,
            _handle,
        }
    }

    /// 创建 Mock 文件保存队列 (用于测试)
    pub fn new_mock() -> Self {
        Self::Mock
    }

    /// 返回任务结果键（如果为真实 Redis 队列）
    pub fn result_key_for_task(&self, task_id: &str) -> Option<String> {
        match self {
            Self::Real { queue_key, .. } => Some(Self::result_key(queue_key, task_id)),
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
                let cancel_key = Self::cancel_key(queue_key, task_id);
                let _: () = conn
                    .set_ex(cancel_key, "1", CANCEL_TTL_SECONDS)
                    .await
                    .map_err(|e| e.to_string())?;
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
                let result_key = Self::result_key(queue_key, task_id);
                let status: Option<String> =
                    conn.get(result_key).await.map_err(|e| e.to_string())?;
                match status {
                    Some(status) => Ok(FileSaveResult::from_status(&status)),
                    None => Ok(None),
                }
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
                let task = Self::normalize_task(task);
                let payload = serde_json::to_string(&task).map_err(|e| e.to_string())?;

                let _: () = conn
                    .lpush(queue_key, payload)
                    .await
                    .map_err(|e| e.to_string())?;
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
                let mut task = Self::normalize_task(task);

                let task_id = if task.task_id.is_empty() {
                    Uuid::new_v4().to_string()
                } else {
                    task.task_id.clone()
                };
                task.task_id = task_id.clone();
                let result_key = task
                    .result_key
                    .clone()
                    .unwrap_or_else(|| Self::result_key(queue_key, &task_id));
                task.result_key = Some(result_key.clone());

                let payload = serde_json::to_string(&task).map_err(|e| e.to_string())?;
                let _: () = conn
                    .lpush(queue_key, payload)
                    .await
                    .map_err(|e| e.to_string())?;

                Self::wait_for_result(conn, &result_key, timeout).await
            }
            Self::Mock => Ok(FileSaveResult::Success),
        }
    }

    fn normalize_task(mut task: FileSaveTask) -> FileSaveTask {
        if task.task_id.is_empty() {
            task.task_id = Uuid::new_v4().to_string();
        }
        if task.max_retries == 0 {
            task.max_retries = DEFAULT_MAX_RETRIES;
        }
        task
    }

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
                .map_err(|e| format!("读取任务结果失败: {}", e))?;

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

    /// 处理任务队列
    async fn process_queue(mut redis: ConnectionManager, queue_key: String) {
        /// 检查是否为超时错误
        fn is_timeout_error(e: &RedisError) -> bool {
            e.to_string().to_lowercase().contains("timed out")
        }
        let processing_key = format!("{}:processing", queue_key);
        let dead_letter_key = format!("{}:dead_letter", queue_key);

        info!(
            "Redis 文件保存任务队列已启动: queue={}, processing={}, dlq={}",
            queue_key, processing_key, dead_letter_key
        );

        if let Err(e) = Self::recover_inflight_tasks(&mut redis, &queue_key, &processing_key).await
        {
            warn!("恢复处理中任务失败: {}", e);
        }

        let mut task_count: usize = 0;
        let mut success_count: usize = 0;
        let mut failure_count: usize = 0;
        let mut cancelled_count: usize = 0;

        loop {
            // 通过 BRPOPLPUSH 原子地将任务转移到 processing 队列，避免进程崩溃导致任务丢失
            let result: Result<Option<String>, _> = redis::cmd("BRPOPLPUSH")
                .arg(&queue_key)
                .arg(&processing_key)
                .arg(5)
                .query_async(&mut redis)
                .await;

            match result {
                Ok(Some(payload)) => {
                    task_count += 1;
                    if let Ok(task) = serde_json::from_str::<FileSaveTask>(&payload) {
                        let cancel_key = Self::cancel_key(&queue_key, &task.task_id);
                        if Self::is_task_cancelled(&mut redis, &cancel_key)
                            .await
                            .unwrap_or(false)
                        {
                            cancelled_count += 1;
                            if let Err(e) = Self::cleanup_task_artifacts(&task).await {
                                warn!("清理已取消任务文件失败 [{}]: {}", task.image_id, e);
                            }
                            if let Err(e) = Self::ack_task_cancelled(
                                &mut redis,
                                &processing_key,
                                &payload,
                                &task,
                                &cancel_key,
                            )
                            .await
                            {
                                error!("确认取消任务失败: {}", e);
                            }
                            continue;
                        }

                        let result = Self::process_task(task.clone()).await;
                        match result {
                            FileSaveResult::Success => {
                                if Self::is_task_cancelled(&mut redis, &cancel_key)
                                    .await
                                    .unwrap_or(false)
                                {
                                    cancelled_count += 1;
                                    if let Err(e) = Self::cleanup_task_artifacts(&task).await {
                                        warn!("清理已取消任务文件失败 [{}]: {}", task.image_id, e);
                                    }
                                    if let Err(e) = Self::ack_task_cancelled(
                                        &mut redis,
                                        &processing_key,
                                        &payload,
                                        &task,
                                        &cancel_key,
                                    )
                                    .await
                                    {
                                        error!("确认取消任务失败: {}", e);
                                    }
                                } else {
                                    success_count += 1;
                                    if let Err(e) = Self::ack_task_success(
                                        &mut redis,
                                        &processing_key,
                                        &payload,
                                        &task,
                                        &cancel_key,
                                    )
                                    .await
                                    {
                                        error!("任务确认成功失败: {}", e);
                                    }
                                }
                            }
                            other => {
                                if Self::is_task_cancelled(&mut redis, &cancel_key)
                                    .await
                                    .unwrap_or(false)
                                {
                                    cancelled_count += 1;
                                    if let Err(e) = Self::cleanup_task_artifacts(&task).await {
                                        warn!("清理已取消任务文件失败 [{}]: {}", task.image_id, e);
                                    }
                                    if let Err(e) = Self::ack_task_cancelled(
                                        &mut redis,
                                        &processing_key,
                                        &payload,
                                        &task,
                                        &cancel_key,
                                    )
                                    .await
                                    {
                                        error!("确认取消任务失败: {}", e);
                                    }
                                } else {
                                    failure_count += 1;
                                    if let Err(e) = Self::handle_task_failure(
                                        &mut redis,
                                        &queue_key,
                                        &processing_key,
                                        &dead_letter_key,
                                        task,
                                        payload,
                                        other,
                                    )
                                    .await
                                    {
                                        error!("处理失败任务时出错: {}", e);
                                    }
                                }
                            }
                        }
                    } else {
                        error!("Failed to deserialize task payload");
                        failure_count += 1;
                        if let Err(e) = Self::move_to_dead_letter(
                            &mut redis,
                            &processing_key,
                            &dead_letter_key,
                            &payload,
                        )
                        .await
                        {
                            error!("无法移动损坏任务到死信队列: {}", e);
                        }
                    }
                }
                Ok(None) => {
                    // 队列为空，继续等待
                    continue;
                }
                Err(e) => {
                    // 超时是正常行为（队列空闲），不记录错误日志
                    if is_timeout_error(&e) {
                        // 静默继续，队列空闲是正常状态
                        continue;
                    }
                    // 其他错误需要记录
                    error!("Redis BRPOP error: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }

            // 定期输出统计
            if task_count > 0 && task_count.is_multiple_of(100) {
                info!(
                    "文件保存任务统计: 总数={}, 成功={}, 失败={}, 取消={}",
                    task_count, success_count, failure_count, cancelled_count
                );
            }
        }
    }

    async fn recover_inflight_tasks(
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

    async fn is_task_cancelled(
        redis: &mut ConnectionManager,
        cancel_key: &str,
    ) -> Result<bool, RedisError> {
        redis.exists(cancel_key).await
    }

    async fn ack_task_success(
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

    async fn ack_task_cancelled(
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

    async fn handle_task_failure(
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
        error!(
            "文件保存任务达到最大重试次数，已移入死信队列: image_id={}",
            task.image_id
        );
        Ok(())
    }

    async fn move_to_dead_letter(
        redis: &mut ConnectionManager,
        processing_key: &str,
        dead_letter_key: &str,
        payload: &str,
    ) -> Result<(), RedisError> {
        let _: () = redis.lpush(dead_letter_key, payload).await?;
        let _: i32 = redis.lrem(processing_key, 1, payload).await?;
        Ok(())
    }

    fn staging_path(path: &str, task_id: &str, label: &str) -> String {
        format!("{}.{}.{}", path, task_id, label)
    }

    async fn cleanup_task_artifacts(task: &FileSaveTask) -> std::io::Result<()> {
        let _ = tokio::fs::remove_file(&task.storage_path).await;
        let _ = tokio::fs::remove_file(&task.thumbnail_path).await;
        let _ = tokio::fs::remove_file(&task.temp_image_path).await;

        let image_stage = Self::staging_path(&task.storage_path, &task.task_id, "imgtmp");
        let thumb_stage = Self::staging_path(&task.thumbnail_path, &task.task_id, "thumbtmp");
        let _ = tokio::fs::remove_file(image_stage).await;
        let _ = tokio::fs::remove_file(thumb_stage).await;
        Ok(())
    }

    /// 处理单个文件保存任务
    #[tracing::instrument(skip(task), fields(image_id = %task.image_id))]
    async fn process_task(task: FileSaveTask) -> FileSaveResult {
        let image_id = &task.image_id;

        let storage_exists = tokio::fs::try_exists(&task.storage_path)
            .await
            .unwrap_or(false);
        let thumbnail_exists = tokio::fs::try_exists(&task.thumbnail_path)
            .await
            .unwrap_or(false);
        if storage_exists && thumbnail_exists {
            let _ = tokio::fs::remove_file(&task.temp_image_path).await;
            return FileSaveResult::Success;
        }

        if !tokio::fs::try_exists(&task.temp_image_path)
            .await
            .unwrap_or(false)
        {
            error!(
                "临时主图不存在且目标文件不完整，无法继续处理 [{}]: {}",
                image_id, task.temp_image_path
            );
            return FileSaveResult::ImageFailed;
        }

        if let Some(parent) = Path::new(&task.storage_path).parent()
            && let Err(e) = tokio::fs::create_dir_all(parent).await
        {
            error!("创建主图目录失败 [{}]: {}", image_id, e);
            return FileSaveResult::ImageFailed;
        }
        if let Some(parent) = Path::new(&task.thumbnail_path).parent()
            && let Err(e) = tokio::fs::create_dir_all(parent).await
        {
            error!("创建缩略图目录失败 [{}]: {}", image_id, e);
            return FileSaveResult::ThumbnailFailed;
        }

        let image_staging = Self::staging_path(&task.storage_path, &task.task_id, "imgtmp");
        let thumb_staging = Self::staging_path(&task.thumbnail_path, &task.task_id, "thumbtmp");

        let _ = tokio::fs::remove_file(&image_staging).await;
        let _ = tokio::fs::remove_file(&thumb_staging).await;

        if let Err(e) = tokio::fs::copy(&task.temp_image_path, &image_staging).await {
            error!("复制主图到暂存文件失败 [{}]: {}", image_id, e);
            let _ = tokio::fs::remove_file(&image_staging).await;
            return FileSaveResult::ImageFailed;
        }

        if let Err(e) = Self::save_file_with_retry(&thumb_staging, &task.thumbnail_data, 3).await {
            error!("保存缩略图暂存文件失败 [{}]: {}", image_id, e);
            let _ = tokio::fs::remove_file(&image_staging).await;
            let _ = tokio::fs::remove_file(&thumb_staging).await;
            return FileSaveResult::ThumbnailFailed;
        }

        if let Err(e) = tokio::fs::rename(&image_staging, &task.storage_path).await {
            error!("原子替换主图失败 [{}]: {}", image_id, e);
            match tokio::fs::copy(&image_staging, &task.storage_path).await {
                Ok(_) => {
                    let _ = tokio::fs::remove_file(&image_staging).await;
                }
                Err(copy_err) => {
                    error!("主图写入失败（copy fallback） [{}]: {}", image_id, copy_err);
                    let _ = tokio::fs::remove_file(&image_staging).await;
                    let _ = tokio::fs::remove_file(&thumb_staging).await;
                    return FileSaveResult::ImageFailed;
                }
            }
        }

        if let Err(e) = tokio::fs::rename(&thumb_staging, &task.thumbnail_path).await {
            error!("原子替换缩略图失败 [{}]: {}", image_id, e);
            match tokio::fs::copy(&thumb_staging, &task.thumbnail_path).await {
                Ok(_) => {
                    let _ = tokio::fs::remove_file(&thumb_staging).await;
                }
                Err(copy_err) => {
                    error!(
                        "缩略图写入失败（copy fallback） [{}]: {}",
                        image_id, copy_err
                    );
                    let _ = tokio::fs::remove_file(&thumb_staging).await;
                    return FileSaveResult::ThumbnailFailed;
                }
            }
        }

        let _ = tokio::fs::remove_file(&task.temp_image_path).await;
        FileSaveResult::Success
    }

    /// 带重试的文件保存
    async fn save_file_with_retry(
        path: &str,
        data: &[u8],
        max_retries: u32,
    ) -> std::io::Result<()> {
        let mut last_error = None;

        for attempt in 1..=max_retries {
            match tokio::fs::write(path, data).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries {
                        let delay = std::time::Duration::from_millis(100 * (2u64.pow(attempt)));
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| std::io::Error::other("文件写入失败")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_queue_creation() {
        // 由于需要 Redis，此处仅做占位
    }
}
