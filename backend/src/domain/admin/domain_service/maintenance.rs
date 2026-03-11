use chrono::Utc;
use sqlx::{MySql, QueryBuilder, Sqlite};
use std::collections::{HashMap, HashSet};
use tracing::{error, info};
use uuid::Uuid;

use super::AdminDomainService;
use crate::audit::log_audit_db;
use crate::db::DatabasePool;
use crate::error::AppError;

fn spawn_maintenance_audit(
    database: DatabasePool,
    admin_user_id: Uuid,
    action: &'static str,
    details: serde_json::Value,
) {
    tokio::spawn(async move {
        log_audit_db(
            &database,
            Some(admin_user_id),
            action,
            "maintenance",
            None,
            None,
            Some(details),
        )
        .await;
    });
}

impl AdminDomainService {
    pub async fn cleanup_deleted_files(
        &self,
        admin_user_id: Uuid,
        admin_email: &str,
    ) -> Result<Vec<String>, AppError> {
        let now = Utc::now();
        let retention_days = self.config.cleanup.deleted_images_retention_days;
        let days_ago = now - chrono::Duration::days(retention_days);
        let database = self.database.clone();

        let result = match &self.database {
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, (Uuid, String)>(
                    "SELECT id, filename FROM images WHERE deleted_at < $1",
                )
                .bind(days_ago)
                .fetch_all(pool)
                .await
            }
            DatabasePool::MySql(pool) => {
                sqlx::query_as::<_, (Uuid, String)>(
                    "SELECT id, filename FROM images WHERE deleted_at < ?",
                )
                .bind(days_ago)
                .fetch_all(pool)
                .await
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as::<_, (Uuid, String)>(
                    "SELECT id, filename FROM images WHERE deleted_at < ?1",
                )
                .bind(days_ago)
                .fetch_all(pool)
                .await
            }
        }
        .map_err(|e| {
            error!("Failed to query cleanup images: {}", e);
            spawn_maintenance_audit(
                database.clone(),
                admin_user_id,
                "admin.maintenance.deleted_files_cleanup.failed",
                serde_json::json!({
                    "admin_email": admin_email,
                    "error": e.to_string(),
                    "result": "failed",
                    "risk_level": "danger",
                }),
            );
            AppError::DatabaseError(e)
        })?;

        let delete_ids: Vec<Uuid> = result.iter().map(|(id, _)| *id).collect();
        let candidate_filenames: Vec<String> = result
            .iter()
            .map(|(_, filename)| filename.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let referenced_filenames: HashSet<String> = match &self.database {
            DatabasePool::Postgres(pool) => {
                sqlx::query_scalar::<_, String>(
                    "SELECT DISTINCT filename
                     FROM images
                     WHERE filename = ANY($1)
                       AND NOT (id = ANY($2))",
                )
                .bind(&candidate_filenames)
                .bind(&delete_ids)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySql(pool) => {
                if candidate_filenames.is_empty() {
                    Vec::new()
                } else {
                    let mut builder = QueryBuilder::<MySql>::new(
                        "SELECT DISTINCT filename FROM images WHERE filename IN (",
                    );
                    {
                        let mut separated = builder.separated(", ");
                        for filename in &candidate_filenames {
                            separated.push_bind(filename);
                        }
                    }
                    builder.push(")");
                    if !delete_ids.is_empty() {
                        builder.push(" AND id NOT IN (");
                        {
                            let mut separated = builder.separated(", ");
                            for image_id in &delete_ids {
                                separated.push_bind(image_id);
                            }
                        }
                        builder.push(")");
                    }
                    builder.build_query_scalar().fetch_all(pool).await?
                }
            }
            DatabasePool::Sqlite(pool) => {
                if candidate_filenames.is_empty() {
                    Vec::new()
                } else {
                    let mut builder = QueryBuilder::<Sqlite>::new(
                        "SELECT DISTINCT filename FROM images WHERE filename IN (",
                    );
                    {
                        let mut separated = builder.separated(", ");
                        for filename in &candidate_filenames {
                            separated.push_bind(filename);
                        }
                    }
                    builder.push(")");
                    if !delete_ids.is_empty() {
                        builder.push(" AND id NOT IN (");
                        {
                            let mut separated = builder.separated(", ");
                            for image_id in &delete_ids {
                                separated.push_bind(image_id);
                            }
                        }
                        builder.push(")");
                    }
                    builder.build_query_scalar().fetch_all(pool).await?
                }
            }
        }
        .into_iter()
        .collect();

        let removable_filenames: HashSet<String> = candidate_filenames
            .into_iter()
            .filter(|filename| !referenced_filenames.contains(filename))
            .collect();

        let mut removed = vec![];
        let mut file_delete_results = HashMap::new();
        for (id, filename) in &result {
            let can_delete_row = if removable_filenames.contains(filename) {
                if let Some(result) = file_delete_results.get(filename) {
                    *result
                } else {
                    let delete_ok = self.storage_manager.delete(filename).await.is_ok();
                    if delete_ok {
                        removed.push(filename.clone());
                    }
                    file_delete_results.insert(filename.clone(), delete_ok);
                    delete_ok
                }
            } else {
                true
            };

            if can_delete_row {
                match &self.database {
                    DatabasePool::Postgres(pool) => {
                        let _ = sqlx::query("DELETE FROM images WHERE id = $1")
                            .bind(id)
                            .execute(pool)
                            .await;
                    }
                    DatabasePool::MySql(pool) => {
                        let _ = sqlx::query("DELETE FROM images WHERE id = ?")
                            .bind(id)
                            .execute(pool)
                            .await;
                    }
                    DatabasePool::Sqlite(pool) => {
                        let _ = sqlx::query("DELETE FROM images WHERE id = ?1")
                            .bind(id)
                            .execute(pool)
                            .await;
                    }
                }
            }
        }

        info!(
            "Cleanup complete: {} images removed by {}",
            removed.len(),
            admin_email
        );

        let removed_count = removed.len();
        spawn_maintenance_audit(
            database,
            admin_user_id,
            "admin.maintenance.deleted_files_cleanup.completed",
            serde_json::json!({
                "admin_email": admin_email,
                "removed_count": removed_count,
                "result": "completed",
                "risk_level": "danger",
            }),
        );

        Ok(removed)
    }

    pub async fn cleanup_expired_images(
        &self,
        admin_user_id: Uuid,
        admin_email: &str,
    ) -> Result<i64, AppError> {
        let now = Utc::now();
        let database = self.database.clone();

        let affected = match &self.database {
            DatabasePool::Postgres(pool) => sqlx::query(
                "UPDATE images SET deleted_at = $1 WHERE expires_at < $1 AND deleted_at IS NULL",
            )
            .bind(now)
            .execute(pool)
            .await
            .map(|result| result.rows_affected() as i64),
            DatabasePool::MySql(pool) => sqlx::query(
                "UPDATE images SET deleted_at = ? WHERE expires_at < ? AND deleted_at IS NULL",
            )
            .bind(now)
            .bind(now)
            .execute(pool)
            .await
            .map(|result| result.rows_affected() as i64),
            DatabasePool::Sqlite(pool) => sqlx::query(
                "UPDATE images SET deleted_at = ?1 WHERE expires_at < ?1 AND deleted_at IS NULL",
            )
            .bind(now)
            .execute(pool)
            .await
            .map(|result| result.rows_affected() as i64),
        }
        .map_err(|e: sqlx::Error| {
            error!("Failed to expire images: {}", e);
            spawn_maintenance_audit(
                database.clone(),
                admin_user_id,
                "admin.maintenance.expired_images_cleanup.failed",
                serde_json::json!({
                    "admin_email": admin_email,
                    "error": e.to_string(),
                    "result": "failed",
                    "risk_level": "warning",
                }),
            );
            AppError::DatabaseError(e)
        })?;
        if affected > 0 {
            info!("Expired images moved to trash: {}", affected);
        }

        spawn_maintenance_audit(
            database,
            admin_user_id,
            "admin.maintenance.expired_images_cleanup.completed",
            serde_json::json!({
                "admin_email": admin_email,
                "affected_count": affected,
                "result": "completed",
                "risk_level": "warning",
            }),
        );

        Ok(affected)
    }
}
