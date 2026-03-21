use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tracing::warn;

const DEFAULT_BACKUP_COMMAND_TIMEOUT: Duration = Duration::from_secs(30 * 60);

pub(super) struct ExternalCommandOutcome {
    pub(super) stderr_excerpt: Option<String>,
}

pub(super) async fn run_streaming_dump_command(
    command_name: &str,
    mut command: Command,
    output_path: &Path,
) -> anyhow::Result<ExternalCommandOutcome> {
    let mut child = spawn_dump_command(command_name, &mut command)?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("{} 未提供可读取的 stdout", command_name))?;
    let stderr_task = capture_stderr_task(command_name, &mut child)?;
    let timeout = backup_command_timeout();
    let mut file = tokio::fs::File::create(output_path).await?;

    match tokio::time::timeout(timeout, tokio::io::copy(&mut stdout, &mut file)).await {
        Ok(Ok(_)) => {
            file.flush().await?;
        }
        Ok(Err(error)) => {
            kill_dump_process(command_name, &mut child).await;
            let _ = collect_stderr_excerpt(stderr_task).await;
            cleanup_backup_file(output_path).await;
            return Err(anyhow::anyhow!(
                "{} 输出流写入失败: {}",
                command_name,
                error
            ));
        }
        Err(_) => {
            kill_dump_process(command_name, &mut child).await;
            let _ = collect_stderr_excerpt(stderr_task).await;
            cleanup_backup_file(output_path).await;
            return Err(anyhow::anyhow!(
                "{} 超时，{} 秒内未完成",
                command_name,
                timeout.as_secs()
            ));
        }
    }

    let stderr_excerpt =
        finalize_dump_command(command_name, child, stderr_task, output_path).await?;
    Ok(ExternalCommandOutcome { stderr_excerpt })
}

pub(super) fn pg_dump_binary() -> anyhow::Result<String> {
    override_or_binary("AVENRIXA_PG_DUMP_BIN", &["pg_dump"])
        .ok_or_else(|| anyhow::anyhow!("未找到 pg_dump"))
}

fn backup_command_timeout() -> Duration {
    std::env::var("AVENRIXA_BACKUP_COMMAND_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map(Duration::from_secs)
        .unwrap_or(DEFAULT_BACKUP_COMMAND_TIMEOUT)
}

fn spawn_dump_command(command_name: &str, command: &mut Command) -> anyhow::Result<Child> {
    command
        .spawn()
        .map_err(|error| anyhow::anyhow!("无法启动 {}: {}", command_name, error))
}

fn capture_stderr_task(
    command_name: &str,
    child: &mut Child,
) -> anyhow::Result<tokio::task::JoinHandle<Vec<u8>>> {
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("{} 未提供可读取的 stderr", command_name))?;
    Ok(tokio::spawn(async move {
        let mut buffer = Vec::new();
        let _ = stderr.read_to_end(&mut buffer).await;
        buffer
    }))
}

async fn finalize_dump_command(
    command_name: &str,
    mut child: Child,
    stderr_task: tokio::task::JoinHandle<Vec<u8>>,
    output_path: &Path,
) -> anyhow::Result<Option<String>> {
    let timeout = backup_command_timeout();
    let status = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) => status,
        Ok(Err(error)) => {
            cleanup_backup_file(output_path).await;
            let _ = collect_stderr_excerpt(stderr_task).await;
            return Err(anyhow::anyhow!("等待 {} 结束失败: {}", command_name, error));
        }
        Err(_) => {
            kill_dump_process(command_name, &mut child).await;
            cleanup_backup_file(output_path).await;
            let _ = collect_stderr_excerpt(stderr_task).await;
            return Err(anyhow::anyhow!(
                "{} 超时，{} 秒内未退出",
                command_name,
                timeout.as_secs()
            ));
        }
    };

    let stderr_excerpt = collect_stderr_excerpt(stderr_task).await;
    if !status.success() {
        cleanup_backup_file(output_path).await;
        return Err(anyhow::anyhow!(
            "{} 执行失败: {}",
            command_name,
            stderr_excerpt.clone().unwrap_or_else(|| status.to_string())
        ));
    }
    if let Some(stderr_excerpt) = stderr_excerpt.as_ref() {
        warn!(
            "{} completed with warnings: {}",
            command_name, stderr_excerpt
        );
    }

    Ok(stderr_excerpt)
}

async fn collect_stderr_excerpt(stderr_task: tokio::task::JoinHandle<Vec<u8>>) -> Option<String> {
    match stderr_task.await {
        Ok(bytes) => process_output_excerpt(&bytes),
        Err(error) => Some(format!("stderr capture failed: {}", error)),
    }
}

async fn kill_dump_process(command_name: &str, child: &mut Child) {
    if let Err(error) = child.kill().await {
        warn!("failed to kill {} after error: {}", command_name, error);
    }
    let _ = child.wait().await;
}

async fn cleanup_backup_file(path: &Path) {
    let _ = tokio::fs::remove_file(path).await;
}

fn override_or_binary(env_key: &str, candidates: &[&str]) -> Option<String> {
    if let Some(path) = std::env::var_os(env_key)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
    {
        return Some(path.to_string_lossy().into_owned());
    }
    find_first_binary(candidates)
}

fn find_first_binary(candidates: &[&str]) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path) {
        for candidate in candidates {
            let full_path = directory.join(candidate);
            if full_path.is_file() {
                return Some(full_path.to_string_lossy().into_owned());
            }
        }
    }
    None
}

fn process_output_excerpt(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let excerpt: String = trimmed.chars().take(1000).collect();
    if trimmed.chars().count() > 1000 {
        Some(format!("{}...(truncated)", excerpt))
    } else {
        Some(excerpt)
    }
}
