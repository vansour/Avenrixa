pub use shared_types::bootstrap::{
    BootstrapDatabaseKind, BootstrapStatusResponse, UpdateBootstrapDatabaseConfigRequest,
    UpdateBootstrapDatabaseConfigResponse,
};

use crate::config::DatabaseKind;

pub fn bootstrap_database_kind_from_config(value: DatabaseKind) -> BootstrapDatabaseKind {
    match value {
        DatabaseKind::Postgres => BootstrapDatabaseKind::Postgres,
    }
}

pub fn config_database_kind_from_bootstrap(value: BootstrapDatabaseKind) -> Option<DatabaseKind> {
    match value {
        BootstrapDatabaseKind::Postgres => Some(DatabaseKind::Postgres),
        _ => None,
    }
}
