mod admin;
mod migrations;
mod pool;
mod schema;
mod state;

pub use admin::{
    ADMIN_USER_ID, INSTALL_STATE_SETTING_KEY, SITE_FAVICON_DATA_URL_SETTING_KEY,
    acquire_installation_lock, create_admin_account_mysql_tx, create_admin_account_sqlite_tx,
    create_admin_account_tx, delete_setting_mysql_tx, delete_setting_sqlite_tx, delete_setting_tx,
    get_setting_value, has_admin_account, has_admin_account_mysql_tx, has_admin_account_sqlite_tx,
    has_admin_account_tx, is_app_installed, is_app_installed_mysql_tx, is_app_installed_sqlite_tx,
    is_app_installed_tx, mark_app_installed_mysql_tx, mark_app_installed_sqlite_tx,
    mark_app_installed_tx, upsert_setting_mysql_tx, upsert_setting_sqlite_tx, upsert_setting_tx,
    validate_admin_bootstrap_config,
};
pub use pool::DatabasePool;
pub use schema::run_migrations;
pub use state::AppState;
