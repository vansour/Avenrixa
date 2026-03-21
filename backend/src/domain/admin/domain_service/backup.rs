mod commands;
mod files;

use chrono::Utc;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{error, info};
use uuid::Uuid;

use self::commands::{pg_dump_binary, run_streaming_dump_command};
use self::files::{
    backup_path, ensure_nonempty_backup_file, file_timestamp, is_valid_backup_filename,
    persist_backup_manifest,
};
use super::AdminDomainService;
use crate::audit::log_audit_db;
use crate::backup_manifest::backup_directory;
use crate::config::DatabaseKind;
use crate::db::DatabasePool;
use crate::error::AppError;
use crate::models::{
    BackupFileSummary, BackupResponse, backup_semantics_from_database_kind, infer_backup_semantics,
};

fn spawn_backup_audit(
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
    pub async fn list_backups(&self) -> Result<Vec<BackupFileSummary>, AppError> {
        let backup_dir = backup_directory();
        if !tokio::fs::try_exists(&backup_dir).await.unwrap_or(false) {
            return Ok(Vec::new());
        }

        let mut directory = tokio::fs::read_dir(&backup_dir).await?;
        let mut backups = Vec::new();

        while let Some(entry) = directory.next_entry().await? {
            let Ok(file_type) = entry.file_type().await else {
                continue;
            };
            if !file_type.is_file() {
                continue;
            }

            let Some(filename) = entry.file_name().to_str().map(ToOwned::to_owned) else {
                continue;
            };
            if !is_valid_backup_filename(&filename) {
                continue;
            }

            let Ok(metadata) = entry.metadata().await else {
                continue;
            };
            let created_at = file_timestamp(&metadata).unwrap_or_else(Utc::now);
            let semantics = infer_backup_semantics(&filename, None);

            backups.push(BackupFileSummary {
                filename,
                created_at,
                size_bytes: metadata.len(),
                semantics,
            });
        }

        backups.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.filename.cmp(&left.filename))
        });

        Ok(backups)
    }

    pub async fn backup_database(
        &self,
        admin_user_id: Uuid,
        admin_email: &str,
    ) -> Result<BackupResponse, AppError> {
        tokio::fs::create_dir_all(backup_directory()).await?;
        let storage_settings = self.storage_manager.active_settings().storage_settings();

        match &self.database {
            DatabasePool::Postgres(_) => {
                let semantics = backup_semantics_from_database_kind(DatabaseKind::Postgres);
                let filename = format!("backup_{}.sql", Uuid::new_v4());
                let backup_path = backup_path(&filename)?;
                let database = self.database.clone();
                let database_url = &self.config.database.url;
                let dump_bin = pg_dump_binary().map_err(AppError::Internal)?;
                let mut command = Command::new(&dump_bin);
                command
                    .arg("--dbname")
                    .arg(database_url)
                    .arg("--format=plain")
                    .arg("--no-owner")
                    .arg("--no-acl")
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());

                let warning_excerpt =
                    match run_streaming_dump_command("pg_dump", command, &backup_path).await {
                        Ok(outcome) => outcome.stderr_excerpt,
                        Err(error) => {
                            error!("pg_dump backup failed: {}", error);
                            spawn_backup_audit(
                                database,
                                admin_user_id,
                                "admin.maintenance.database_backup.failed",
                                serde_json::json!({
                                    "admin_email": admin_email,
                                    "error": error.to_string(),
                                    "result": "failed",
                                    "risk_level": "info",
                                    "database_kind": "postgresql",
                                }),
                            );
                            return Err(AppError::Internal(error));
                        }
                    };

                let backup_size_bytes = match ensure_nonempty_backup_file(&backup_path).await {
                    Ok(size) => size,
                    Err(error) => {
                        let _ = tokio::fs::remove_file(&backup_path).await;
                        spawn_backup_audit(
                            self.database.clone(),
                            admin_user_id,
                            "admin.maintenance.database_backup.failed",
                            serde_json::json!({
                                "admin_email": admin_email,
                                "filename": filename,
                                "error": error.to_string(),
                                "result": "failed",
                                "risk_level": "info",
                                "database_kind": "postgresql",
                            }),
                        );
                        return Err(AppError::Internal(error));
                    }
                };

                let created_at = Utc::now();
                if let Err(error) = persist_backup_manifest(
                    &filename,
                    DatabaseKind::Postgres,
                    semantics.clone(),
                    created_at,
                    &storage_settings,
                )
                .await
                {
                    let _ = tokio::fs::remove_file(&backup_path).await;
                    spawn_backup_audit(
                        self.database.clone(),
                        admin_user_id,
                        "admin.maintenance.database_backup.failed",
                        serde_json::json!({
                            "admin_email": admin_email,
                            "filename": filename,
                            "error": error.to_string(),
                            "result": "failed",
                            "risk_level": "info",
                            "database_kind": "postgresql",
                        }),
                    );
                    return Err(AppError::Internal(error));
                }

                info!("Database backup created: {} by {}", filename, admin_email);
                spawn_backup_audit(
                    self.database.clone(),
                    admin_user_id,
                    "admin.maintenance.database_backup.created",
                    serde_json::json!({
                        "admin_email": admin_email,
                        "filename": filename,
                        "result": "completed",
                        "risk_level": "info",
                        "database_kind": "postgresql",
                        "backup_kind": semantics.backup_kind.clone(),
                        "backup_scope": semantics.backup_scope.clone(),
                        "restore_mode": semantics.restore_mode.clone(),
                        "ui_restore_supported": semantics.ui_restore_supported,
                        "backup_size_bytes": backup_size_bytes,
                        "warning_excerpt": warning_excerpt,
                    }),
                );

                Ok(BackupResponse {
                    filename,
                    created_at,
                    semantics,
                })
            }
        }
    }

    pub async fn download_backup(
        &self,
        admin_user_id: Uuid,
        admin_email: &str,
        filename: &str,
    ) -> Result<Vec<u8>, AppError> {
        let backup_path = backup_path(filename)?;
        let bytes = match tokio::fs::read(&backup_path).await {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(AppError::BackupNotFound);
            }
            Err(error) => return Err(AppError::IoError(error)),
        };

        spawn_backup_audit(
            self.database.clone(),
            admin_user_id,
            "admin.maintenance.database_backup.downloaded",
            serde_json::json!({
                "admin_email": admin_email,
                "filename": filename,
                "result": "completed",
                "risk_level": "info",
                "size_bytes": bytes.len(),
            }),
        );

        Ok(bytes)
    }

    pub async fn delete_backup(
        &self,
        admin_user_id: Uuid,
        admin_email: &str,
        filename: &str,
    ) -> Result<(), AppError> {
        let backup_path = backup_path(filename)?;
        let size_bytes = match tokio::fs::metadata(&backup_path).await {
            Ok(metadata) => metadata.len(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                spawn_backup_audit(
                    self.database.clone(),
                    admin_user_id,
                    "admin.maintenance.database_backup.delete_failed",
                    serde_json::json!({
                        "admin_email": admin_email,
                        "filename": filename,
                        "error": "backup file not found",
                        "result": "failed",
                        "risk_level": "danger",
                    }),
                );
                return Err(AppError::BackupNotFound);
            }
            Err(error) => {
                spawn_backup_audit(
                    self.database.clone(),
                    admin_user_id,
                    "admin.maintenance.database_backup.delete_failed",
                    serde_json::json!({
                        "admin_email": admin_email,
                        "filename": filename,
                        "error": error.to_string(),
                        "result": "failed",
                        "risk_level": "danger",
                    }),
                );
                return Err(AppError::IoError(error));
            }
        };

        match tokio::fs::remove_file(&backup_path).await {
            Ok(()) => {
                spawn_backup_audit(
                    self.database.clone(),
                    admin_user_id,
                    "admin.maintenance.database_backup.deleted",
                    serde_json::json!({
                        "admin_email": admin_email,
                        "filename": filename,
                        "size_bytes": size_bytes,
                        "result": "completed",
                        "risk_level": "danger",
                    }),
                );
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                spawn_backup_audit(
                    self.database.clone(),
                    admin_user_id,
                    "admin.maintenance.database_backup.delete_failed",
                    serde_json::json!({
                        "admin_email": admin_email,
                        "filename": filename,
                        "error": "backup file not found",
                        "result": "failed",
                        "risk_level": "danger",
                    }),
                );
                Err(AppError::BackupNotFound)
            }
            Err(error) => {
                spawn_backup_audit(
                    self.database.clone(),
                    admin_user_id,
                    "admin.maintenance.database_backup.delete_failed",
                    serde_json::json!({
                        "admin_email": admin_email,
                        "filename": filename,
                        "error": error.to_string(),
                        "result": "failed",
                        "risk_level": "danger",
                    }),
                );
                Err(AppError::IoError(error))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DatabaseKind};
    use crate::models::ComponentStatus;
    use crate::runtime_settings::{RuntimeSettings, StorageBackend};
    use crate::storage_backend::StorageManager;
    use std::ffi::OsString;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::sync::{Arc, OnceLock};
    use tempfile::TempDir;
    use tokio::sync::Mutex;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct ScopedEnv {
        previous: Vec<(String, Option<OsString>)>,
    }

    impl ScopedEnv {
        fn set(key: &str, value: impl Into<OsString>) -> Self {
            let previous = std::env::var_os(key);
            #[allow(unused_unsafe)]
            unsafe {
                std::env::set_var(key, value.into());
            }
            Self {
                previous: vec![(key.to_string(), previous)],
            }
        }

        fn and_set(mut self, key: &str, value: impl Into<OsString>) -> Self {
            let previous = std::env::var_os(key);
            #[allow(unused_unsafe)]
            unsafe {
                std::env::set_var(key, value.into());
            }
            self.previous.push((key.to_string(), previous));
            self
        }
    }

    impl Drop for ScopedEnv {
        fn drop(&mut self) {
            for (key, previous) in self.previous.drain(..).rev() {
                #[allow(unused_unsafe)]
                unsafe {
                    match previous {
                        Some(previous) => std::env::set_var(&key, previous),
                        None => std::env::remove_var(&key),
                    }
                }
            }
        }
    }

    fn sample_runtime_settings(local_storage_path: String) -> RuntimeSettings {
        RuntimeSettings {
            site_name: "Avenrixa".to_string(),
            storage_backend: StorageBackend::Local,
            local_storage_path,
            mail_enabled: false,
            mail_smtp_host: String::new(),
            mail_smtp_port: 587,
            mail_smtp_user: None,
            mail_smtp_password: None,
            mail_from_email: String::new(),
            mail_from_name: String::new(),
            mail_link_base_url: String::new(),
        }
    }

    fn build_postgres_backup_service(temp_dir: &TempDir) -> AdminDomainService {
        let mut config = Config::default();
        config.database.kind = DatabaseKind::Postgres;
        config.database.url = "postgres://localhost/test".to_string();
        config.storage.path = temp_dir
            .path()
            .join("storage")
            .to_string_lossy()
            .into_owned();

        let database = DatabasePool::Postgres(
            sqlx::PgPool::connect_lazy("postgres://localhost/test")
                .expect("lazy postgres pool should be created"),
        );

        let storage_manager = Arc::new(StorageManager::new(sample_runtime_settings(
            config.storage.path.clone(),
        )));

        AdminDomainService::new(database, None, config, storage_manager)
    }

    fn write_script(path: &Path, body: &str) {
        std::fs::write(path, body).expect("script should be written");
        let mut permissions = std::fs::metadata(path)
            .expect("script metadata should load")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("script should be executable");
    }

    #[tokio::test]
    async fn postgres_backup_streams_stdout_to_file_and_persists_manifest() {
        let _env_guard = env_lock().lock().await;
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let backup_dir = temp_dir.path().join("backups");
        let script_path = temp_dir.path().join("fake-pg-dump.sh");
        write_script(
            &script_path,
            "#!/bin/sh\nprintf 'CREATE TABLE demo(id int);\\n'\n",
        );
        let _env = ScopedEnv::set("AVENRIXA_BACKUP_DIR", backup_dir.as_os_str())
            .and_set("AVENRIXA_PG_DUMP_BIN", script_path.as_os_str());

        let service = build_postgres_backup_service(&temp_dir);
        let response = service
            .backup_database(Uuid::new_v4(), "admin@example.com")
            .await
            .expect("backup should succeed");

        let backup_file = backup_dir.join(&response.filename);
        let manifest_file = backup_dir.join(format!("{}.manifest.json", response.filename));
        let contents = tokio::fs::read_to_string(&backup_file)
            .await
            .expect("backup file should exist");

        assert_eq!(contents, "CREATE TABLE demo(id int);\\n");
        assert!(
            tokio::fs::try_exists(&manifest_file)
                .await
                .expect("manifest should exist")
        );
    }

    #[tokio::test]
    async fn postgres_backup_timeout_removes_partial_file() {
        let _env_guard = env_lock().lock().await;
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let backup_dir = temp_dir.path().join("backups");
        let script_path = temp_dir.path().join("fake-pg-dump-timeout.sh");
        write_script(
            &script_path,
            "#!/bin/sh\nprintf 'partial dump'; sleep 2; printf 'late tail'\\n",
        );
        let _env = ScopedEnv::set("AVENRIXA_BACKUP_DIR", backup_dir.as_os_str())
            .and_set("AVENRIXA_PG_DUMP_BIN", script_path.as_os_str())
            .and_set("AVENRIXA_BACKUP_COMMAND_TIMEOUT_SECS", "1");

        let service = build_postgres_backup_service(&temp_dir);
        let error = service
            .backup_database(Uuid::new_v4(), "admin@example.com")
            .await
            .expect_err("backup timeout should fail");

        assert!(matches!(error, AppError::Internal(_)));
        let mut entries = tokio::fs::read_dir(&backup_dir)
            .await
            .expect("backup dir should be readable");
        assert!(
            entries
                .next_entry()
                .await
                .expect("backup dir iteration should succeed")
                .is_none(),
            "timed out backup should not leave partial files"
        );
    }
}
