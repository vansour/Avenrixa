pub use shared_types::bootstrap::{
    BootstrapDatabaseKind, BootstrapStatusResponse, UpdateBootstrapDatabaseConfigRequest,
    UpdateBootstrapDatabaseConfigResponse,
};

use crate::config::DatabaseKind;

pub fn bootstrap_database_kind_from_config(value: DatabaseKind) -> BootstrapDatabaseKind {
    match value {
        DatabaseKind::Postgres => BootstrapDatabaseKind::Postgres,
        DatabaseKind::MySql => BootstrapDatabaseKind::MySql,
        DatabaseKind::Sqlite => BootstrapDatabaseKind::Sqlite,
    }
}

pub fn config_database_kind_from_bootstrap(value: BootstrapDatabaseKind) -> Option<DatabaseKind> {
    match value {
        BootstrapDatabaseKind::Postgres => Some(DatabaseKind::Postgres),
        BootstrapDatabaseKind::MySql => Some(DatabaseKind::MySql),
        BootstrapDatabaseKind::Sqlite => Some(DatabaseKind::Sqlite),
        BootstrapDatabaseKind::Unknown => None,
    }
}
