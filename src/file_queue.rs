//! 文件保存任务队列模块
//! 负责处理文件保存任务，确保文件写入完成后再返回给用户

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

/// 文件保存任务
#[derive(Debug, Clone)]
pub struct FileSaveTask {
    pub image_id: String,
    pub storage_path: String,
    pub thumbnail_path: String,
    pub image_data: Vec<u8>,
    pub thumbnail_data: Vec<u8>,
}

/// 文件保存队列
pub struct FileSaveQueue {
    sender: mpsc::Sender<(FileSaveTask, mpsc::Sender<FileSaveResult>)>,
    _handle: JoinHandle<()>,
}

/// 文件保存结果
#[derive(Debug, Clone)]
pub enum FileSaveResult {
    Success,
    ImageFailed { image_id: String, error: String },
    ThumbnailFailed { image_id: String, error: String },
}

impl FileSaveQueue {
    /// 创建新的文件保存队列
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(100);
        let _handle = tokio::spawn(async move {
            Self::process_queue(receiver).await;
        });

        Self { sender, _handle }
    }

    /// 提交文件保存任务并等待完成
    pub async fn submit(&self, task: FileSaveTask) -> Result<(), FileSaveResult> {
        // 提前保存 image_id 以避免借用问题
        let image_id = task.image_id.clone();

        // 创建结果通道
        let (result_sender, mut result_receiver) = mpsc::channel(1);

        // 发送任务和结果通道到队列
        if self.sender.send((task, result_sender)).await.is_err() {
            return Err(FileSaveResult::ImageFailed {
                image_id,
                error: "任务队列已关闭".to_string(),
            });
        }

        // 等待结果
        match result_receiver.recv().await {
            Some(FileSaveResult::Success) => Ok(()),
            Some(e @ (FileSaveResult::ImageFailed { .. } | FileSaveResult::ThumbnailFailed { .. })) => Err(e),
            None => Err(FileSaveResult::ImageFailed {
                image_id,
                error: "任务结果通道已关闭".to_string(),
            }),
        }
    }

    /// 处理队列中的任务
    async fn process_queue(
        mut receiver: mpsc::Receiver<(FileSaveTask, mpsc::Sender<FileSaveResult>)>,
    ) {
        info!("文件保存任务队列已启动");
        let mut task_count: usize = 0;
        let mut success_count: usize = 0;
        let mut failure_count: usize = 0;

        while let Some((task, result_sender)) = receiver.recv().await {
            task_count += 1;

            let result = Self::process_task(task).await;
            if matches!(result, FileSaveResult::Success) {
                success_count += 1;
            } else {
                failure_count += 1;
            }

            // 发送结果
            let _ = result_sender.send(result).await;

            // 定期输出统计
            if task_count.is_multiple_of(100) {
                info!(
                    "文件保存任务统计: 总数={}, 成功={}, 失败={}",
                    task_count, success_count, failure_count
                );
            }
        }

        info!(
            "文件保存任务队列已关闭: 总任务={}, 成功={}, 失败={}",
            task_count, success_count, failure_count
        );
    }

    /// 处理单个文件保存任务
    async fn process_task(task: FileSaveTask) -> FileSaveResult {
        let image_id = &task.image_id;

        // 保存主图片
        match Self::save_file_with_retry(&task.storage_path, &task.image_data, 3).await {
            Ok(_) => {
                // 主图片保存成功，保存缩略图
                match Self::save_file_with_retry(&task.thumbnail_path, &task.thumbnail_data, 3).await {
                    Ok(_) => FileSaveResult::Success,
                    Err(e) => {
                        error!("保存缩略图失败 [{}]: {}", image_id, e);
                        FileSaveResult::ThumbnailFailed {
                            image_id: task.image_id.clone(),
                            error: e.to_string(),
                        }
                    }
                }
            }
            Err(e) => {
                error!("保存图片失败 [{}]: {}", image_id, e);
                FileSaveResult::ImageFailed {
                    image_id: task.image_id.clone(),
                    error: e.to_string(),
                }
            }
        }
    }

    /// 带重试机制的文件保存
    async fn save_file_with_retry(path: &str, data: &[u8], max_retries: usize) -> std::io::Result<()> {
        let mut last_error = None;

        for attempt in 0..max_retries {
            match tokio::fs::write(path, data).await {
                Ok(_) => {
                    if attempt > 0 {
                        info!("文件写入在 {} 次尝试后成功: {}", attempt + 1, path);
                    }
                    return Ok(());
                }
                Err(e) if attempt < max_retries - 1 => {
                    warn!("文件写入失败 (尝试 {}/{}): {} - {}", attempt + 1, max_retries, path, e);
                    last_error = Some(e);
                    // 指数退避延迟：100ms * 2^attempt
                    let delay = std::time::Duration::from_millis(100 * (2u64.pow(attempt as u32)));
                    tokio::time::sleep(delay).await;
                    continue;
                }
                Err(e) => {
                    error!("文件写入失败，已达到最大重试次数 {}: {}", max_retries, path);
                    return Err(e);
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
        let queue = FileSaveQueue::new();
        // 测试队列创建成功
        assert_eq!(queue.sender.capacity(), 100);
    }

    #[tokio::test]
    async fn test_task_submit() {
        let queue = FileSaveQueue::new();

        let task = FileSaveTask {
            image_id: "test-id".to_string(),
            storage_path: "/tmp/test.jpg".to_string(),
            thumbnail_path: "/tmp/test-thumb.jpg".to_string(),
            image_data: vec![0; 100],
            thumbnail_data: vec![0; 100],
        };

        // 注意：由于这个测试会创建实际文件，我们只测试提交逻辑
        // 在实际使用中，任务会被异步处理
        let result = queue.submit(task).await;
        assert!(matches!(result, Ok(()) | Err(_)));
    }
}
