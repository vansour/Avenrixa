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
        DatabasePool::MySql(pool) => {
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
                     VALUES (?, ?, ?, ?, 0, NULL, ?, ?, ?)
                     ON DUPLICATE KEY UPDATE
                         storage_snapshot = VALUES(storage_snapshot),
                         attempts = 0,
                         last_error = NULL,
                         next_attempt_at = VALUES(next_attempt_at),
                         updated_at = VALUES(updated_at)",
                )
                .bind(Uuid::new_v4())
                .bind(file_key)
                .bind(&signature)
                .bind(&storage_snapshot)
                .bind(now)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }
        DatabasePool::Sqlite(pool) => {
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
                     VALUES (?1, ?2, ?3, ?4, 0, NULL, ?5, ?5, ?5)
                     ON CONFLICT(storage_signature, file_key)
                     DO UPDATE SET
                         storage_snapshot = excluded.storage_snapshot,
                         attempts = 0,
                         last_error = NULL,
                         next_attempt_at = excluded.next_attempt_at,
                         updated_at = excluded.updated_at",
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

        if file_key_is_still_referenced(database, &job.file_key).await? {
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
        DatabasePool::MySql(pool) => {
            sqlx::query_as::<_, StorageCleanupJobRow>(
                "SELECT id, file_key, storage_snapshot, attempts
                 FROM storage_cleanup_jobs
                 WHERE next_attempt_at <= ?
                 ORDER BY next_attempt_at ASC, created_at ASC
                 LIMIT ?",
            )
            .bind(now)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        DatabasePool::Sqlite(pool) => {
            sqlx::query_as::<_, StorageCleanupJobRow>(
                "SELECT id, file_key, storage_snapshot, attempts
                 FROM storage_cleanup_jobs
                 WHERE next_attempt_at <= ?1
                 ORDER BY next_attempt_at ASC, created_at ASC
                 LIMIT ?2",
            )
            .bind(now)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(jobs)
}

async fn file_key_is_still_referenced(
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
        DatabasePool::MySql(pool) => {
            sqlx::query_scalar::<_, i32>(
                "SELECT 1
                 FROM images
                 WHERE (filename = ? OR thumbnail = ?)
                   AND status = ?
                 LIMIT 1",
            )
            .bind(file_key)
            .bind(file_key)
            .bind(active_status)
            .fetch_optional(pool)
            .await?
        }
        DatabasePool::Sqlite(pool) => {
            sqlx::query_scalar::<_, i32>(
                "SELECT 1
                 FROM images
                 WHERE (filename = ?1 OR thumbnail = ?1)
                   AND status = ?2
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

async fn delete_storage_cleanup_job(database: &DatabasePool, job_id: Uuid) -> Result<(), AppError> {
    match database {
        DatabasePool::Postgres(pool) => {
            sqlx::query("DELETE FROM storage_cleanup_jobs WHERE id = $1")
                .bind(job_id)
                .execute(pool)
                .await?;
        }
        DatabasePool::MySql(pool) => {
            sqlx::query("DELETE FROM storage_cleanup_jobs WHERE id = ?")
                .bind(job_id)
                .execute(pool)
                .await?;
        }
        DatabasePool::Sqlite(pool) => {
            sqlx::query("DELETE FROM storage_cleanup_jobs WHERE id = ?1")
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
        DatabasePool::MySql(pool) => {
            sqlx::query(
                "UPDATE storage_cleanup_jobs
                 SET attempts = ?,
                     last_error = ?,
                     next_attempt_at = ?,
                     updated_at = ?
                 WHERE id = ?",
            )
            .bind(attempts)
            .bind(&last_error)
            .bind(next_attempt_at)
            .bind(now)
            .bind(job_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::Sqlite(pool) => {
            sqlx::query(
                "UPDATE storage_cleanup_jobs
                 SET attempts = ?1,
                     last_error = ?2,
                     next_attempt_at = ?3,
                     updated_at = ?4
                 WHERE id = ?5",
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
    const MAX_ERROR_LENGTH: usize = 2000;
    let trimmed = message.trim();
    if trimmed.chars().count() <= MAX_ERROR_LENGTH {
        return trimmed.to_string();
    }

    trimmed.chars().take(MAX_ERROR_LENGTH).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{DatabasePool, run_migrations};
    use crate::runtime_settings::StorageBackend;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn setup_sqlite_database() -> (tempfile::TempDir, tempfile::TempDir, DatabasePool) {
        let database_dir = tempfile::tempdir().expect("database temp dir should be created");
        let storage_dir = tempfile::tempdir().expect("storage temp dir should be created");
        let database_path = database_dir.path().join("storage-cleanup.db");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&database_path)
                    .create_if_missing(true)
                    .foreign_keys(true),
            )
            .await
            .expect("sqlite pool should be created");
        let database = DatabasePool::Sqlite(pool);
        run_migrations(&database)
            .await
            .expect("sqlite migrations should succeed");

        (database_dir, storage_dir, database)
    }

    fn local_snapshot(storage_dir: &tempfile::TempDir) -> StorageSettingsSnapshot {
        StorageSettingsSnapshot {
            storage_backend: StorageBackend::Local,
            local_storage_path: storage_dir.path().to_string_lossy().into_owned(),
            s3_endpoint: None,
            s3_region: None,
            s3_bucket: None,
            s3_prefix: None,
            s3_access_key: None,
            s3_secret_key: None,
            s3_force_path_style: true,
        }
    }

    async fn queued_job_count(database: &DatabasePool, file_key: &str) -> i64 {
        let DatabasePool::Sqlite(pool) = database else {
            panic!("test helper expects sqlite database");
        };

        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)
             FROM storage_cleanup_jobs
             WHERE file_key = ?1",
        )
        .bind(file_key)
        .fetch_one(pool)
        .await
        .expect("queued job count should load")
    }

    async fn queued_job_attempts(database: &DatabasePool, file_key: &str) -> i64 {
        let DatabasePool::Sqlite(pool) = database else {
            panic!("test helper expects sqlite database");
        };

        sqlx::query_scalar::<_, i64>(
            "SELECT attempts
             FROM storage_cleanup_jobs
             WHERE file_key = ?1",
        )
        .bind(file_key)
        .fetch_one(pool)
        .await
        .expect("queued job attempts should load")
    }

    async fn mark_job_ready_now(database: &DatabasePool, file_key: &str) {
        let DatabasePool::Sqlite(pool) = database else {
            panic!("test helper expects sqlite database");
        };

        sqlx::query(
            "UPDATE storage_cleanup_jobs
             SET next_attempt_at = ?1
             WHERE file_key = ?2",
        )
        .bind(Utc::now() - Duration::seconds(1))
        .bind(file_key)
        .execute(pool)
        .await
        .expect("queued job should be made ready");
    }

    async fn insert_active_image_reference(
        database: &DatabasePool,
        filename: &str,
        thumbnail: Option<&str>,
    ) {
        let DatabasePool::Sqlite(pool) = database else {
            panic!("test helper expects sqlite database");
        };

        sqlx::query(
            "INSERT INTO images (
                 id,
                 user_id,
                 filename,
                 thumbnail,
                 size,
                 hash,
                 format,
                 views,
                 status,
                 expires_at,
                 created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, NULL, ?9)",
        )
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .bind(filename)
        .bind(thumbnail)
        .bind(123_i64)
        .bind(format!("{:064x}", 42))
        .bind("png")
        .bind(ImageStatus::Active.as_str())
        .bind(Utc::now())
        .execute(pool)
        .await
        .expect("active image reference should be inserted");
    }

    #[tokio::test]
    async fn process_pending_storage_cleanup_jobs_retries_failed_delete_until_it_succeeds() {
        let (_database_dir, storage_dir, database) = setup_sqlite_database().await;
        let snapshot = local_snapshot(&storage_dir);
        let file_key = "retry-success.png".to_string();
        let file_path = storage_dir.path().join(&file_key);

        tokio::fs::create_dir_all(&file_path)
            .await
            .expect("directory placeholder should force remove_file failure");
        enqueue_storage_cleanup_jobs(&database, &snapshot, std::slice::from_ref(&file_key))
            .await
            .expect("cleanup job should be enqueued");

        let first_processed = process_pending_storage_cleanup_jobs(&database, 10)
            .await
            .expect("first cleanup pass should complete");

        assert_eq!(first_processed, 1);
        assert_eq!(queued_job_count(&database, &file_key).await, 1);
        assert_eq!(queued_job_attempts(&database, &file_key).await, 1);

        tokio::fs::remove_dir(&file_path)
            .await
            .expect("directory placeholder should be removed");
        tokio::fs::write(&file_path, b"payload")
            .await
            .expect("physical object should exist before retry");
        mark_job_ready_now(&database, &file_key).await;

        let second_processed = process_pending_storage_cleanup_jobs(&database, 10)
            .await
            .expect("second cleanup pass should complete");

        assert_eq!(second_processed, 1);
        assert_eq!(queued_job_count(&database, &file_key).await, 0);
        assert!(
            !tokio::fs::try_exists(&file_path)
                .await
                .expect("file existence check should succeed"),
            "physical object should be removed after successful retry"
        );
    }

    #[tokio::test]
    async fn process_pending_storage_cleanup_jobs_drops_job_when_file_is_referenced_again() {
        let (_database_dir, storage_dir, database) = setup_sqlite_database().await;
        let snapshot = local_snapshot(&storage_dir);
        let file_key = "re-referenced.png".to_string();
        let file_path = storage_dir.path().join(&file_key);

        tokio::fs::write(&file_path, b"payload")
            .await
            .expect("physical object should exist before cleanup pass");
        enqueue_storage_cleanup_jobs(&database, &snapshot, std::slice::from_ref(&file_key))
            .await
            .expect("cleanup job should be enqueued");
        insert_active_image_reference(&database, &file_key, None).await;

        let processed = process_pending_storage_cleanup_jobs(&database, 10)
            .await
            .expect("cleanup pass should complete");

        assert_eq!(processed, 1);
        assert_eq!(queued_job_count(&database, &file_key).await, 0);
        assert!(
            tokio::fs::try_exists(&file_path)
                .await
                .expect("file existence check should succeed"),
            "re-referenced object must not be deleted"
        );
    }

    #[tokio::test]
    async fn process_pending_storage_cleanup_jobs_drops_job_when_thumbnail_is_referenced_again() {
        let (_database_dir, storage_dir, database) = setup_sqlite_database().await;
        let snapshot = local_snapshot(&storage_dir);
        let file_key = "re-referenced-thumbnail.webp".to_string();
        let file_path = storage_dir.path().join(&file_key);

        tokio::fs::write(&file_path, b"payload")
            .await
            .expect("thumbnail object should exist before cleanup pass");
        enqueue_storage_cleanup_jobs(&database, &snapshot, std::slice::from_ref(&file_key))
            .await
            .expect("cleanup job should be enqueued");
        insert_active_image_reference(&database, "source.png", Some(&file_key)).await;

        let processed = process_pending_storage_cleanup_jobs(&database, 10)
            .await
            .expect("cleanup pass should complete");

        assert_eq!(processed, 1);
        assert_eq!(queued_job_count(&database, &file_key).await, 0);
        assert!(
            tokio::fs::try_exists(&file_path)
                .await
                .expect("thumbnail existence check should succeed"),
            "re-referenced thumbnail must not be deleted"
        );
    }
}
