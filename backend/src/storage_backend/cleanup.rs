use std::collections::BTreeSet;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::backup_manifest::storage_signature;
use crate::db::DatabasePool;
use crate::error::AppError;
use crate::models::ImageStatus;
use crate::runtime_settings::StorageSettingsSnapshot;

use super::ops;

#[derive(Debug, sqlx::FromRow)]
struct StorageCleanupJobRow {
    id: Uuid,
    file_key: String,
    storage_snapshot: String,
    attempts: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct MediaBlobCleanupRow {
    ref_count: i64,
    status: String,
}

pub async fn enqueue_storage_cleanup_jobs(
    database: &DatabasePool,
    snapshot: &StorageSettingsSnapshot,
    file_keys: &[String],
) -> Result<u64, AppError> {
    let unique_file_keys: Vec<String> = file_keys
        .iter()
        .map(|file_key| file_key.trim())
        .filter(|file_key| !file_key.is_empty())
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if unique_file_keys.is_empty() {
        return Ok(0);
    }

    let now = Utc::now();
    let signature = storage_signature(snapshot);
    let storage_snapshot =
        serde_json::to_string(snapshot).map_err(|error| AppError::Internal(error.into()))?;

    match database {
        DatabasePool::Postgres(pool) => {
            for file_key in &unique_file_keys {
                sqlx::query(
                    "INSERT INTO storage_cleanup_jobs (
                         id,
                         file_key,
                         storage_signature,
                         storage_snapshot,
                         attempts,
                         last_error,
                         next_attempt_at,
                         created_at,
                         updated_at
                     )
                     VALUES ($1, $2, $3, $4, 0, NULL, $5, $5, $5)
                     ON CONFLICT (storage_signature, file_key)
                     DO UPDATE SET
                         storage_snapshot = EXCLUDED.storage_snapshot,
                         attempts = 0,
                         last_error = NULL,
                         next_attempt_at = EXCLUDED.next_attempt_at,
                         updated_at = EXCLUDED.updated_at",
                )
                .bind(Uuid::new_v4())
                .bind(file_key)
                .bind(&signature)
                .bind(&storage_snapshot)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }
    }

    Ok(unique_file_keys.len() as u64)
}

pub async fn process_pending_storage_cleanup_jobs(
    database: &DatabasePool,
    limit: usize,
) -> Result<u64, AppError> {
    let jobs = load_pending_storage_cleanup_jobs(database, Utc::now(), limit).await?;
    if jobs.is_empty() {
        return Ok(0);
    }

    let mut processed = 0_u64;
    for job in jobs {
        processed += 1;

        if let Some(blob) = load_media_blob_cleanup_row(database, &job.file_key).await? {
            if blob.ref_count > 0 || blob.status == "deleted" {
                delete_storage_cleanup_job(database, job.id).await?;
                continue;
            }
        } else if legacy_image_reference_exists(database, &job.file_key).await? {
            delete_storage_cleanup_job(database, job.id).await?;
            continue;
        }

        let storage_snapshot: StorageSettingsSnapshot =
            match serde_json::from_str(&job.storage_snapshot) {
                Ok(storage_snapshot) => storage_snapshot,
                Err(error) => {
                    record_storage_cleanup_failure(
                        database,
                        job.id,
                        job.attempts + 1,
                        &format!("无法解析存储清理任务快照: {}", error),
                    )
                    .await?;
                    continue;
                }
            };

        match ops::delete_with_storage_snapshot(&storage_snapshot, &job.file_key).await {
            Ok(()) => {
                mark_media_blob_deleted(database, &job.file_key).await?;
                delete_storage_cleanup_job(database, job.id).await?;
            }
            Err(error) => {
                record_storage_cleanup_failure(
                    database,
                    job.id,
                    job.attempts + 1,
                    &error.to_string(),
                )
                .await?;
            }
        }
    }

    Ok(processed)
}

async fn load_pending_storage_cleanup_jobs(
    database: &DatabasePool,
    now: DateTime<Utc>,
    limit: usize,
) -> Result<Vec<StorageCleanupJobRow>, AppError> {
    let limit = (limit.max(1)).min(i32::MAX as usize) as i32;
    let jobs = match database {
        DatabasePool::Postgres(pool) => {
            sqlx::query_as::<_, StorageCleanupJobRow>(
                "SELECT id, file_key, storage_snapshot, attempts
                 FROM storage_cleanup_jobs
                 WHERE next_attempt_at <= $1
                 ORDER BY next_attempt_at ASC, created_at ASC
                 LIMIT $2",
            )
            .bind(now)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(jobs)
}

async fn load_media_blob_cleanup_row(
    database: &DatabasePool,
    file_key: &str,
) -> Result<Option<MediaBlobCleanupRow>, AppError> {
    let row = match database {
        DatabasePool::Postgres(pool) => {
            sqlx::query_as::<_, MediaBlobCleanupRow>(
                "SELECT ref_count, status
                 FROM media_blobs
                 WHERE storage_key = $1
                 LIMIT 1",
            )
            .bind(file_key)
            .fetch_optional(pool)
            .await?
        }
    };

    Ok(row)
}

async fn legacy_image_reference_exists(
    database: &DatabasePool,
    file_key: &str,
) -> Result<bool, AppError> {
    let active_status = ImageStatus::Active.as_str();
    let exists = match database {
        DatabasePool::Postgres(pool) => {
            sqlx::query_scalar::<_, i32>(
                "SELECT 1
                 FROM images
                 WHERE (filename = $1 OR thumbnail = $1)
                   AND status = $2
                 LIMIT 1",
            )
            .bind(file_key)
            .bind(active_status)
            .fetch_optional(pool)
            .await?
        }
    };

    Ok(exists.is_some())
}

async fn mark_media_blob_deleted(database: &DatabasePool, file_key: &str) -> Result<(), AppError> {
    match database {
        DatabasePool::Postgres(pool) => {
            sqlx::query(
                "UPDATE media_blobs
                 SET status = 'deleted',
                     updated_at = NOW()
                 WHERE storage_key = $1",
            )
            .bind(file_key)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

async fn delete_storage_cleanup_job(database: &DatabasePool, job_id: Uuid) -> Result<(), AppError> {
    match database {
        DatabasePool::Postgres(pool) => {
            sqlx::query("DELETE FROM storage_cleanup_jobs WHERE id = $1")
                .bind(job_id)
                .execute(pool)
                .await?;
        }
    }

    Ok(())
}

async fn record_storage_cleanup_failure(
    database: &DatabasePool,
    job_id: Uuid,
    attempts: i64,
    error_message: &str,
) -> Result<(), AppError> {
    let now = Utc::now();
    let next_attempt_at = now + retry_backoff(attempts);
    let last_error = trim_error_message(error_message);

    match database {
        DatabasePool::Postgres(pool) => {
            sqlx::query(
                "UPDATE storage_cleanup_jobs
                 SET attempts = $1,
                     last_error = $2,
                     next_attempt_at = $3,
                     updated_at = $4
                 WHERE id = $5",
            )
            .bind(attempts)
            .bind(&last_error)
            .bind(next_attempt_at)
            .bind(now)
            .bind(job_id)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

fn retry_backoff(attempts: i64) -> Duration {
    let clamped_attempts = attempts.clamp(1, 6) as u32;
    let seconds = 30_i64.saturating_mul(1_i64 << (clamped_attempts - 1));
    Duration::seconds(seconds.min(15 * 60))
}

fn trim_error_message(message: &str) -> String {
    const MAX_ERROR_LENGTH: usize = 200;
    let trimmed = message.trim();
    if trimmed.chars().count() <= MAX_ERROR_LENGTH {
        return trimmed.to_string();
    }

    trimmed.chars().take(MAX_ERROR_LENGTH).collect()
}
