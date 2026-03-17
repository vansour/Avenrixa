use crate::db::{
    AppState, INSTALL_STATE_SETTING_KEY, SITE_FAVICON_DATA_URL_SETTING_KEY,
    acquire_installation_lock, create_admin_account_mysql_tx, create_admin_account_sqlite_tx,
    create_admin_account_tx, delete_admin_account_mysql_tx, delete_admin_account_sqlite_tx,
    delete_admin_account_tx, has_admin_account_mysql_tx, has_admin_account_sqlite_tx,
    has_admin_account_tx, is_app_installed_mysql_tx, is_app_installed_sqlite_tx,
    is_app_installed_tx, mark_app_installed_mysql_tx, mark_app_installed_sqlite_tx,
    mark_app_installed_tx, upsert_setting_mysql_tx, upsert_setting_sqlite_tx, upsert_setting_tx,
};
use crate::error::AppError;
use crate::models::User;
use crate::runtime_settings::{
    RuntimeSettings, persist_settings_mysql_tx, persist_settings_sqlite_tx, persist_settings_tx,
};

pub(super) struct InstallPersistenceInput<'a> {
    pub(super) validated_settings: &'a RuntimeSettings,
    pub(super) admin_email: &'a str,
    pub(super) admin_password: &'a str,
    pub(super) favicon_data_url: Option<&'a str>,
}

pub(super) struct InstallRollbackInput<'a> {
    pub(super) previous_settings: &'a RuntimeSettings,
    pub(super) previous_install_state: Option<&'a str>,
    pub(super) previous_favicon: Option<&'a str>,
}

pub(super) async fn persist_installation(
    state: &AppState,
    input: &InstallPersistenceInput<'_>,
) -> Result<User, AppError> {
    match &state.database {
        crate::db::DatabasePool::Postgres(pool) => {
            persist_installation_postgres(
                pool,
                input.validated_settings,
                input.admin_email,
                input.admin_password,
                input.favicon_data_url,
            )
            .await
        }
        crate::db::DatabasePool::MySql(pool) => {
            persist_installation_mysql(
                pool,
                input.validated_settings,
                input.admin_email,
                input.admin_password,
                input.favicon_data_url,
            )
            .await
        }
        crate::db::DatabasePool::Sqlite(pool) => {
            persist_installation_sqlite(
                pool,
                input.validated_settings,
                input.admin_email,
                input.admin_password,
                input.favicon_data_url,
            )
            .await
        }
    }
}

pub(super) async fn rollback_failed_installation(
    state: &AppState,
    input: &InstallRollbackInput<'_>,
) -> Result<(), AppError> {
    match &state.database {
        crate::db::DatabasePool::Postgres(pool) => {
            rollback_failed_installation_postgres(
                pool,
                input.previous_settings,
                input.previous_install_state,
                input.previous_favicon,
            )
            .await
        }
        crate::db::DatabasePool::MySql(pool) => {
            rollback_failed_installation_mysql(
                pool,
                input.previous_settings,
                input.previous_install_state,
                input.previous_favicon,
            )
            .await
        }
        crate::db::DatabasePool::Sqlite(pool) => {
            rollback_failed_installation_sqlite(
                pool,
                input.previous_settings,
                input.previous_install_state,
                input.previous_favicon,
            )
            .await
        }
    }
}

async fn persist_installation_postgres(
    pool: &sqlx::PgPool,
    validated_settings: &RuntimeSettings,
    admin_email: &str,
    admin_password: &str,
    favicon_data_url: Option<&str>,
) -> Result<User, AppError> {
    let mut tx = pool.begin().await?;
    acquire_installation_lock(&mut tx).await?;

    if is_app_installed_tx(&mut tx).await? || has_admin_account_tx(&mut tx).await? {
        return Err(AppError::AppAlreadyInstalled);
    }

    persist_settings_tx(&mut tx, validated_settings).await?;
    restore_optional_setting_tx(&mut tx, SITE_FAVICON_DATA_URL_SETTING_KEY, favicon_data_url)
        .await?;

    let user = create_admin_account_tx(&mut tx, admin_email, admin_password)
        .await
        .map_err(AppError::Internal)?;
    mark_app_installed_tx(&mut tx).await?;
    tx.commit().await?;
    Ok(user)
}

async fn persist_installation_mysql(
    pool: &sqlx::MySqlPool,
    validated_settings: &RuntimeSettings,
    admin_email: &str,
    admin_password: &str,
    favicon_data_url: Option<&str>,
) -> Result<User, AppError> {
    let mut tx = pool.begin().await?;

    if is_app_installed_mysql_tx(&mut tx).await? || has_admin_account_mysql_tx(&mut tx).await? {
        return Err(AppError::AppAlreadyInstalled);
    }

    persist_settings_mysql_tx(&mut tx, validated_settings).await?;
    restore_optional_setting_mysql_tx(&mut tx, SITE_FAVICON_DATA_URL_SETTING_KEY, favicon_data_url)
        .await?;

    let user = create_admin_account_mysql_tx(&mut tx, admin_email, admin_password)
        .await
        .map_err(AppError::Internal)?;
    mark_app_installed_mysql_tx(&mut tx).await?;
    tx.commit().await?;
    Ok(user)
}

async fn persist_installation_sqlite(
    pool: &sqlx::SqlitePool,
    validated_settings: &RuntimeSettings,
    admin_email: &str,
    admin_password: &str,
    favicon_data_url: Option<&str>,
) -> Result<User, AppError> {
    let mut tx = pool.begin().await?;

    if is_app_installed_sqlite_tx(&mut tx).await? || has_admin_account_sqlite_tx(&mut tx).await? {
        return Err(AppError::AppAlreadyInstalled);
    }

    persist_settings_sqlite_tx(&mut tx, validated_settings).await?;
    restore_optional_setting_sqlite_tx(
        &mut tx,
        SITE_FAVICON_DATA_URL_SETTING_KEY,
        favicon_data_url,
    )
    .await?;

    let user = create_admin_account_sqlite_tx(&mut tx, admin_email, admin_password)
        .await
        .map_err(AppError::Internal)?;
    mark_app_installed_sqlite_tx(&mut tx).await?;
    tx.commit().await?;
    Ok(user)
}

async fn rollback_failed_installation_postgres(
    pool: &sqlx::PgPool,
    previous_settings: &RuntimeSettings,
    previous_install_state: Option<&str>,
    previous_favicon: Option<&str>,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    acquire_installation_lock(&mut tx).await?;
    persist_settings_tx(&mut tx, previous_settings).await?;
    restore_optional_setting_tx(&mut tx, SITE_FAVICON_DATA_URL_SETTING_KEY, previous_favicon)
        .await?;
    restore_optional_setting_tx(&mut tx, INSTALL_STATE_SETTING_KEY, previous_install_state).await?;
    delete_admin_account_tx(&mut tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn rollback_failed_installation_mysql(
    pool: &sqlx::MySqlPool,
    previous_settings: &RuntimeSettings,
    previous_install_state: Option<&str>,
    previous_favicon: Option<&str>,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    persist_settings_mysql_tx(&mut tx, previous_settings).await?;
    restore_optional_setting_mysql_tx(&mut tx, SITE_FAVICON_DATA_URL_SETTING_KEY, previous_favicon)
        .await?;
    restore_optional_setting_mysql_tx(&mut tx, INSTALL_STATE_SETTING_KEY, previous_install_state)
        .await?;
    delete_admin_account_mysql_tx(&mut tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn rollback_failed_installation_sqlite(
    pool: &sqlx::SqlitePool,
    previous_settings: &RuntimeSettings,
    previous_install_state: Option<&str>,
    previous_favicon: Option<&str>,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    persist_settings_sqlite_tx(&mut tx, previous_settings).await?;
    restore_optional_setting_sqlite_tx(
        &mut tx,
        SITE_FAVICON_DATA_URL_SETTING_KEY,
        previous_favicon,
    )
    .await?;
    restore_optional_setting_sqlite_tx(&mut tx, INSTALL_STATE_SETTING_KEY, previous_install_state)
        .await?;
    delete_admin_account_sqlite_tx(&mut tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn restore_optional_setting_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    key: &str,
    value: Option<&str>,
) -> Result<(), AppError> {
    match value {
        Some(value) => upsert_setting_tx(tx, key, value).await?,
        None => crate::db::delete_setting_tx(tx, key).await?,
    }
    Ok(())
}

async fn restore_optional_setting_mysql_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    key: &str,
    value: Option<&str>,
) -> Result<(), AppError> {
    match value {
        Some(value) => upsert_setting_mysql_tx(tx, key, value).await?,
        None => crate::db::delete_setting_mysql_tx(tx, key).await?,
    }
    Ok(())
}

async fn restore_optional_setting_sqlite_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    key: &str,
    value: Option<&str>,
) -> Result<(), AppError> {
    match value {
        Some(value) => upsert_setting_sqlite_tx(tx, key, value).await?,
        None => crate::db::delete_setting_sqlite_tx(tx, key).await?,
    }
    Ok(())
}
