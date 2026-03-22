use tracing::error;

use crate::error::AppError;
use crate::storage_backend::StorageManager;

use super::super::RuntimeSettings;
use super::super::store::persist_settings_tx;
use super::RuntimeSettingsService;

pub(super) struct PersistAndApplyInput<'a> {
    pub(super) previous: &'a RuntimeSettings,
    pub(super) validated: &'a RuntimeSettings,
    pub(super) storage_manager: &'a StorageManager,
}

pub(super) async fn persist_and_apply(
    service: &RuntimeSettingsService,
    input: &PersistAndApplyInput<'_>,
) -> Result<RuntimeSettings, AppError> {
    input
        .storage_manager
        .validate_runtime_settings(input.validated)
        .await?;

    match &service.pool {
        crate::db::DatabasePool::Postgres(pool) => {
            let mut tx = pool.begin().await?;
            persist_settings_tx(&mut tx, input.validated).await?;
            tx.commit().await?;
        }
    }

    if let Err(apply_error) = input
        .storage_manager
        .apply_runtime_settings(input.validated.clone())
        .await
    {
        let rollback_result =
            rollback_after_apply_failure(service, input.previous, input.storage_manager).await;
        return match rollback_result {
            Ok(()) => Err(apply_error),
            Err(rollback_error) => {
                error!(
                    "runtime settings apply failed and rollback failed: apply={}, rollback={}",
                    apply_error, rollback_error
                );
                Err(AppError::Internal(anyhow::anyhow!(
                    "运行时设置应用失败，且回滚失败: apply={}, rollback={}",
                    apply_error,
                    rollback_error
                )))
            }
        };
    }

    service.invalidate_cache().await;
    service.get_runtime_settings().await
}

async fn rollback_after_apply_failure(
    service: &RuntimeSettingsService,
    previous: &RuntimeSettings,
    storage_manager: &StorageManager,
) -> Result<(), AppError> {
    match &service.pool {
        crate::db::DatabasePool::Postgres(pool) => {
            let mut tx = pool.begin().await?;
            persist_settings_tx(&mut tx, previous).await?;
            tx.commit().await?;
        }
    }

    service.invalidate_cache().await;
    storage_manager
        .apply_runtime_settings(previous.clone())
        .await?;
    service.invalidate_cache().await;
    Ok(())
}
