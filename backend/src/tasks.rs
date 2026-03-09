//! 清理任务模块
//! 负责定期清理过期图片

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;

/// 清理过期图片
pub async fn cleanup_expired_images(
    pool: &PgPool,
    retention_days: i64,
    images_path: &str,
    thumbnail_path: &str,
) -> Result<u64> {
    let days_ago = chrono::Utc::now() - chrono::Duration::days(retention_days);

    let images: Vec<(uuid::Uuid, String)> = sqlx::query_as::<_, (uuid::Uuid, String)>(
        "SELECT id, filename FROM images WHERE deleted_at < $1",
    )
    .bind(days_ago)
    .fetch_all(pool)
    .await?;

    info!("Found {} expired images to clean up", images.len());

    let mut removed_count = 0;
    for (id, filename) in &images {
        let file_storage_path = format!("{}/{}", images_path, filename);
        let file_thumbnail_path = format!("{}/{}.jpg", thumbnail_path, id);

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
