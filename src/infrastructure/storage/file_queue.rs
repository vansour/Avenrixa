//! 文件保存任务队列模块
//! 负责处理文件保存任务，确保文件写入完成后再返回给用户

use redis::AsyncCommands;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};
use serde::{Deserialize, Serialize};
use redis::aio::ConnectionManager;

/// 文件保存任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSaveTask {
    pub image_id: String,
    pub storage_path: String,
    pub thumbnail_path: String,
    pub temp_image_path: String,
    pub thumbnail_data: Vec<u8>, // 缩略图通常很小，可以保留在内存中
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
#[derive(Debug, Clone)]
pub enum FileSaveResult {
    Success,
    ImageFailed,
    ThumbnailFailed,
}

impl FileSaveQueue {
    /// 创建新的文件保存队列
    pub fn new(redis: ConnectionManager, queue_key: String) -> Self {
        let redis_clone = redis.clone();
        let queue_key_clone = queue_key.clone();

        let _handle = tokio::spawn(async move {
            Self::process_queue(redis_clone, queue_key_clone).await;
        });

        Self::Real { redis, queue_key, _handle }
    }

    /// 创建 Mock 文件保存队列 (用于测试)
    pub fn new_mock() -> Self {
        Self::Mock
    }

    /// 提交文件保存任务并存入 Redis
    pub async fn submit(&self, task: FileSaveTask) -> Result<(), String> {
        match self {
            Self::Real { redis, queue_key, .. } => {
                let mut conn = redis.clone();
                let payload = serde_json::to_string(&task).map_err(|e| e.to_string())?;

                let _: () = conn.lpush(queue_key, payload).await.map_err(|e| e.to_string())?;
                Ok(())
            }
            Self::Mock => {
                info!("Mock file save task submitted: {}", task.image_id);
                Ok(())
            }
        }
    }

    /// 处理任务队列
    async fn process_queue(
        mut redis: ConnectionManager,
        queue_key: String,
    ) {
        info!("Redis 文件保存任务队列已启动: {}", queue_key);
        let mut task_count: usize = 0;
        let mut success_count: usize = 0;
        let mut failure_count: usize = 0;

        // 启动时尝试恢复之前的任务（如果使用的是持久化队列，LPUSH/BRPOP 已经保证了这一点）
        // 但为了更健壮，我们可以检查是否有未处理的任务

        loop {
            // 使用 BRPOP 阻塞获取任务
            let result: Result<Option<(String, String)>, _> = redis.brpop(&queue_key, 5.0).await;

            match result {
                Ok(Some((_, payload))) => {
                    task_count += 1;
                    if let Ok(task) = serde_json::from_str::<FileSaveTask>(&payload) {
                        let result = Self::process_task(task).await;
                        match result {
                            FileSaveResult::Success => success_count += 1,
                            _ => failure_count += 1,
                        }
                    } else {
                        error!("Failed to deserialize task payload");
                        failure_count += 1;
                    }
                }
                Ok(None) => {
                    // 超时，继续循环
                    continue;
                }
                Err(e) => {
                    error!("Redis BRPOP error: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }

            // 定期输出统计
            if task_count > 0 && task_count.is_multiple_of(100) {
                info!(
                    "文件保存任务统计: 总数={}, 成功={}, 失败={}",
                    task_count, success_count, failure_count
                );
            }
        }
    }

    /// 处理单个文件保存任务
    #[tracing::instrument(skip(task), fields(image_id = %task.image_id))]
    async fn process_task(task: FileSaveTask) -> FileSaveResult {
        let image_id = &task.image_id;

        // 移动主图片文件（从临时目录到存储目录）
        match tokio::fs::rename(&task.temp_image_path, &task.storage_path).await {
            Ok(_) => {
                // 主图片移动成功，保存缩略图
                match Self::save_file_with_retry(&task.thumbnail_path, &task.thumbnail_data, 3).await {
                    Ok(_) => FileSaveResult::Success,
                    Err(e) => {
                        error!("保存缩略图失败 [{}]: {}", image_id, e);
                        FileSaveResult::ThumbnailFailed
                    }
                }
            }
            Err(e) => {
                error!("移动临时文件到存储目录失败 [{}]: {}", image_id, e);
                // 尝试复制（如果 rename 跨文件系统失败）
                match tokio::fs::copy(&task.temp_image_path, &task.storage_path).await {
                    Ok(_) => {
                        let _ = tokio::fs::remove_file(&task.temp_image_path).await;
                        // 复制成功，保存缩略图
                        match Self::save_file_with_retry(&task.thumbnail_path, &task.thumbnail_data, 3).await {
                            Ok(_) => FileSaveResult::Success,
                            Err(e) => {
                                error!("保存缩略图失败 [{}]: {}", image_id, e);
                                FileSaveResult::ThumbnailFailed
                            }
                        }
                    }
                    Err(copy_err) => {
                        error!("复制临时文件到存储目录失败 [{}]: {}", image_id, copy_err);
                        FileSaveResult::ImageFailed
                    }
                }
            }
        }
    }

    /// 带重试的文件保存
    async fn save_file_with_retry(path: &str, data: &[u8], max_retries: u32) -> std::io::Result<()> {
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

        Err(last_error.unwrap_or_else(|| {
            std::io::Error::other("文件写入失败")
        }))
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
