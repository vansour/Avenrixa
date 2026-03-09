//! 文件保存任务队列模块
//! 负责处理文件保存任务，确保文件写入完成后再返回给用户

mod model;
mod queue;
mod storage;
mod worker;

use tokio::time::Duration;

pub(super) const DEFAULT_MAX_RETRIES: u8 = 3;
pub(super) const RESULT_POLL_INTERVAL: Duration = Duration::from_millis(100);
pub(super) const RESULT_TTL_SECONDS: u64 = 300;
pub(super) const CANCEL_TTL_SECONDS: u64 = 600;

pub(super) fn result_key(queue_key: &str, task_id: &str) -> String {
    format!("{}:result:{}", queue_key, task_id)
}

pub(super) fn cancel_key(queue_key: &str, task_id: &str) -> String {
    format!("{}:cancel:{}", queue_key, task_id)
}

pub(super) fn staging_path(path: &str, task_id: &str, label: &str) -> String {
    format!("{}.{}.{}", path, task_id, label)
}

pub use model::{FileSaveResult, FileSaveTask};
pub use queue::FileSaveQueue;
