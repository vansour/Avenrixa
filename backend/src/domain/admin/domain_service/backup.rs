use chrono::Utc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{error, info};
use uuid::Uuid;

use super::AdminDomainService;
use crate::audit::log_audit;
use crate::error::AppError;
use crate::models::BackupResponse;

impl AdminDomainService {
    pub async fn backup_database(
        &self,
        admin_user_id: Uuid,
        admin_username: &str,
    ) -> Result<BackupResponse, AppError> {
        let filename = format!("backup_{}.sql", Uuid::new_v4());
        let backup_dir = "/data/backup";
        let backup_path = format!("{}/{}", backup_dir, filename);

        tokio::fs::create_dir_all(backup_dir).await?;

        let database_url = &self.config.database.url;
        let child = tokio::process::Command::new("pg_dump")
            .arg("--dbname")
            .arg(database_url)
            .arg("--format=plain")
            .arg("--no-owner")
            .arg("--no-acl")
            .stdout(std::process::Stdio::piped())
            .spawn();

        let mut child = match child {
            Ok(child) => child,
            Err(e) => {
                error!("Failed to spawn pg_dump process: {}", e);
                return Err(AppError::Internal(anyhow::anyhow!(
                    "Failed to execute pg_dump: {}",
                    e
                )));
            }
        };

        let mut stdout = child.stdout.take().expect("Failed to capture stdout");
        let mut buffer = Vec::new();
        let _ = stdout.read_to_end(&mut buffer).await;

        let mut file = tokio::fs::File::create(&backup_path).await?;
        file.write_all(&buffer).await?;

        let status = child.wait().await;
        match status {
            Ok(status) => {
                if !status.success() {
                    error!("pg_dump failed with status: {}", status);
                    let _ = tokio::fs::remove_file(&backup_path).await;
                    return Err(AppError::Internal(anyhow::anyhow!(
                        "pg_dump failed with exit code: {}",
                        status
                    )));
                }
            }
            Err(e) => {
                error!("Failed to wait for pg_dump: {}", e);
                let _ = tokio::fs::remove_file(&backup_path).await;
                return Err(AppError::Internal(anyhow::anyhow!(
                    "pg_dump wait error: {}",
                    e
                )));
            }
        }

        info!(
            "Database backup created: {} by {}",
            filename, admin_username
        );

        let pool = self.pool.clone();
        let admin_id = admin_user_id;
        let filename_clone = filename.clone();
        tokio::spawn(async move {
            let _ = log_audit(
                &pool,
                Some(admin_id),
                "backup_created",
                "backup",
                None,
                None,
                Some(serde_json::json!({"filename": filename_clone})),
            )
            .await;
        });

        Ok(BackupResponse {
            filename,
            created_at: Utc::now(),
        })
    }
}
