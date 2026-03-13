//! 清理任务模块
//! 负责定期清理历史已删除图片和过期图片

use anyhow::Result;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use tracing::info;

/// 清理过期图片
pub async fn cleanup_expired_images(
    pool: &PgPool,
    _retention_days: i64,
    images_path: &str,
) -> Result<u64> {
    let images: Vec<(uuid::Uuid, String)> = sqlx::query_as::<_, (uuid::Uuid, String)>(
        "SELECT id, filename FROM images WHERE deleted_at IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;

    info!("Found {} legacy deleted images to purge", images.len());

    let delete_ids: Vec<uuid::Uuid> = images.iter().map(|(id, _)| *id).collect();
    let candidate_filenames: Vec<String> = images
        .iter()
        .map(|(_, filename)| filename.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let referenced_filenames: HashSet<String> = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT filename
         FROM images
         WHERE filename = ANY($1)
           AND NOT (id = ANY($2))",
    )
    .bind(&candidate_filenames)
    .bind(&delete_ids)
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect();

    let mut removed_count = 0;
    let mut file_delete_results = HashMap::new();
    for (id, filename) in &images {
        let file_storage_path = format!("{}/{}", images_path, filename);

        let can_delete_row = if referenced_filenames.contains(filename) {
            true
        } else if let Some(result) = file_delete_results.get(filename) {
            *result
        } else {
            let delete_ok = tokio::fs::remove_file(&file_storage_path).await.is_ok();
            file_delete_results.insert(filename.clone(), delete_ok);
            delete_ok
        };

        if can_delete_row {
            let _ = sqlx::query("DELETE FROM images WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await;
            removed_count += 1;
        }
    }

    info!(
        "Deleted-image purge complete: {} images removed",
        removed_count
    );
    Ok(removed_count)
}

/// 永久删除已过期图片
pub async fn delete_expired_images(pool: &PgPool, images_path: &str) -> Result<u64> {
    let now = chrono::Utc::now();
    let images: Vec<(uuid::Uuid, String)> = sqlx::query_as::<_, (uuid::Uuid, String)>(
        "SELECT id, filename
         FROM images
         WHERE expires_at < $1
           AND deleted_at IS NULL",
    )
    .bind(now)
    .fetch_all(pool)
    .await?;

    info!("Found {} expired images to delete", images.len());

    let delete_ids: Vec<uuid::Uuid> = images.iter().map(|(id, _)| *id).collect();
    let candidate_filenames: Vec<String> = images
        .iter()
        .map(|(_, filename)| filename.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let referenced_filenames: HashSet<String> = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT filename
         FROM images
         WHERE filename = ANY($1)
           AND NOT (id = ANY($2))",
    )
    .bind(&candidate_filenames)
    .bind(&delete_ids)
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect();

    let mut removed_count = 0_u64;
    let mut file_delete_results = HashMap::new();
    for (id, filename) in &images {
        let file_storage_path = format!("{}/{}", images_path, filename);

        let can_delete_row = if referenced_filenames.contains(filename) {
            true
        } else if let Some(result) = file_delete_results.get(filename) {
            *result
        } else {
            let delete_ok = tokio::fs::remove_file(&file_storage_path).await.is_ok();
            file_delete_results.insert(filename.clone(), delete_ok);
            delete_ok
        };

        if can_delete_row {
            let _ = sqlx::query("DELETE FROM images WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await;
            removed_count += 1;
        }
    }

    if removed_count > 0 {
        info!("Expired images permanently deleted: {}", removed_count);
    }

    Ok(removed_count)
}
