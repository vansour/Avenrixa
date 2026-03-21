use sqlx::PgPool;

#[derive(Clone)]
pub enum DatabasePool {
    Postgres(PgPool),
}
