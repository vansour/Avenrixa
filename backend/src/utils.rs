use std::time::Duration;
use tokio::fs;
use tracing::{error, info, warn};

/// 异步写入文件，带重试机制（指数退避）
pub async fn write_file_with_retry(
    path: &str,
    data: &[u8],
    max_retries: usize,
) -> std::io::Result<()> {
    for attempt in 0..max_retries {
        match fs::write(path, data).await {
            Ok(_) => {
                if attempt > 0 {
                    info!("File write succeeded after {} attempts", attempt + 1);
                }
                return Ok(());
            }
            Err(e) if attempt < max_retries - 1 => {
                warn!(
                    "Failed to write file (attempt {}/{}): {}",
                    attempt + 1,
                    max_retries,
                    e
                );
                let delay = Duration::from_millis(100 * (2u64.pow(attempt as u32)));
                tokio::time::sleep(delay).await;
                continue;
            }
            Err(e) => {
                error!("Failed to write file after {} attempts: {}", max_retries, e);
                return Err(e);
            }
        }
    }
    Ok(())
}
