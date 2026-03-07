//! 清理任务模块
//! 负责定期清理过期图片和临时文件

use anyhow::Result;
use sqlx::PgPool;
use tracing::{info, warn};

/// 清理过期图片
#[allow(dead_code)]
pub async fn cleanup_expired_images(
    pool: &PgPool,
    retention_days: i64,
    storage_path: &str,
) -> Result<u64> {
    let days_ago = chrono::Utc::now() - chrono::Duration::days(retention_days);
    let images_dir = format!("{}/images", storage_path);
    let thumbnail_dir = format!("{}/thumbnails", storage_path);

    let images: Vec<(uuid::Uuid, String)> = sqlx::query_as::<_, (uuid::Uuid, String)>(
        "SELECT id, filename FROM images WHERE deleted_at < $1",
    )
    .bind(days_ago)
    .fetch_all(pool)
    .await?;

    info!("Found {} expired images to clean up", images.len());

    let mut removed_count = 0;
    for (id, filename) in &images {
        let file_storage_path = format!("{}/{}", images_dir, filename);
        let file_thumbnail_path = format!("{}/{}.jpg", thumbnail_dir, id);

        let file_removed = tokio::fs::remove_file(&file_storage_path).await.is_ok();
        let thumb_removed = tokio::fs::remove_file(&file_thumbnail_path).await.is_ok();

        if file_removed || thumb_removed {
            let _ = sqlx::query("DELETE FROM images WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await;
            removed_count += 1;
        }
    }

    info!("Cleanup complete: {} images removed", removed_count);
    Ok(removed_count)
}

/// 将过期图片移至回收站
#[allow(dead_code)]
pub async fn move_expired_to_trash(pool: &PgPool) -> Result<u64> {
    let now = chrono::Utc::now();

    let result = sqlx::query(
        "UPDATE images SET deleted_at = $1 WHERE expires_at < $1 AND deleted_at IS NULL",
    )
    .bind(now)
    .execute(pool)
    .await?;

    if result.rows_affected() > 0 {
        info!("Expired images moved to trash: {}", result.rows_affected());
    }

    Ok(result.rows_affected())
}

/// 清理临时文件
#[allow(dead_code)]
pub async fn cleanup_temp_files(storage_path: &str) -> Result<()> {
    let mut entries = tokio::fs::read_dir(storage_path).await?;

    while let Some(entry) = entries.next_entry().await? {
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        // 匹配临时文件模式
        if filename_str.contains(".tmp") {
            let file_path = entry.path();
            if let Err(e) = tokio::fs::remove_file(&file_path).await {
                warn!("Failed to remove temp file {}: {}", filename_str, e);
            }
        }
    }

    info!("Temp files cleanup completed");
    Ok(())
}
