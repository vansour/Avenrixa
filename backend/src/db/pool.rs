use crate::config::DatabaseKind;
use sqlx::{PgPool, SqlitePool};

#[derive(Clone)]
pub enum DatabasePool {
    Postgres(PgPool),
    #[allow(dead_code)]
    Sqlite(SqlitePool),
}

impl DatabasePool {
    pub fn kind(&self) -> DatabaseKind {
        match self {
            Self::Postgres(_) => DatabaseKind::Postgres,
            Self::Sqlite(_) => DatabaseKind::Sqlite,
        }
    }

    pub fn as_postgres(&self) -> Option<&PgPool> {
        match self {
            Self::Postgres(pool) => Some(pool),
            Self::Sqlite(_) => None,
        }
    }

    pub fn postgres(&self) -> anyhow::Result<&PgPool> {
        self.as_postgres().ok_or_else(|| {
            anyhow::anyhow!(
                "当前数据库后端为 {}，尚未接入 PostgreSQL 专用运行时",
                self.kind().as_str()
            )
        })
    }
}
