use sqlx::{MySqlPool, PgPool, SqlitePool};

#[derive(Clone)]
pub enum DatabasePool {
    Postgres(PgPool),
    MySql(MySqlPool),
    #[allow(dead_code)]
    Sqlite(SqlitePool),
}
