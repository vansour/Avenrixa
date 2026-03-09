use chrono::Utc;
use tracing::{error, info};
use uuid::Uuid;

use super::AdminDomainService;
use crate::audit::log_audit;
use crate::error::AppError;

impl AdminDomainService {
    pub async fn cleanup_deleted_files(
        &self,
        admin_user_id: Uuid,
        admin_username: &str,
    ) -> Result<Vec<String>, AppError> {
        let now = Utc::now();
        let retention_days = self.config.cleanup.deleted_images_retention_days;
        let days_ago = now - chrono::Duration::days(retention_days);

        let result = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, filename FROM images WHERE deleted_at < $1",
        )
        .bind(days_ago)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to query cleanup images: {}", e);
            let pool = self.pool.clone();
            let error_msg = e.to_string();
            let admin_id = admin_user_id;
            tokio::spawn(async move {
                let _ = log_audit(
                    &pool,
                    Some(admin_id),
                    "cleanup_failed",
                    "cleanup",
                    None,
                    None,
                    Some(serde_json::json!({"error": error_msg})),
                )
                .await;
            });
            AppError::DatabaseError(e)
        })?;

        let mut removed = vec![];
        for (id, filename) in &result {
            if self.storage_manager.delete(filename).await.is_ok() {
                let _ = sqlx::query("DELETE FROM images WHERE id = $1")
                    .bind(id)
                    .execute(&self.pool)
                    .await;
                removed.push(filename.clone());
            }
        }

        info!(
            "Cleanup complete: {} images removed by {}",
            removed.len(),
            admin_username
        );

        let removed_count = removed.len();
        let pool = self.pool.clone();
        tokio::spawn(async move {
            let _ = log_audit(
                &pool,
                Some(admin_user_id),
                "cleanup_completed",
                "cleanup",
                None,
                None,
                Some(serde_json::json!({"removed_count": removed_count})),
            )
            .await;
        });

        Ok(removed)
    }

    pub async fn cleanup_expired_images(&self, admin_user_id: Uuid) -> Result<i64, AppError> {
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE images SET deleted_at = $1 WHERE expires_at < $1 AND deleted_at IS NULL",
        )
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to expire images: {}", e);
            let pool = self.pool.clone();
            let error_msg = e.to_string();
            tokio::spawn(async move {
                let _ = log_audit(
                    &pool,
                    Some(admin_user_id),
                    "expire_failed",
                    "expiry",
                    None,
                    None,
                    Some(serde_json::json!({"error": error_msg})),
                )
                .await;
            });
            AppError::DatabaseError(e)
        })?;

        let affected = result.rows_affected() as i64;
        if affected > 0 {
            info!("Expired images moved to trash: {}", affected);
        }

        let pool = self.pool.clone();
        tokio::spawn(async move {
            let _ = log_audit(
                &pool,
                Some(admin_user_id),
                "expire_completed",
                "expiry",
                None,
                None,
                Some(serde_json::json!({"affected_count": affected})),
            )
            .await;
        });

        Ok(affected)
    }

    pub async fn approve_images(&self, image_ids: &[Uuid], approved: bool) -> Result<(), AppError> {
        let status = if approved { "active" } else { "pending" };

        sqlx::query("UPDATE images SET status = $1 WHERE id = ANY($2)")
            .bind(status)
            .bind(image_ids)
            .execute(&self.pool)
            .await?;

        info!("Images approved: {:?} -> {}", image_ids, status);
        Ok(())
    }
}
