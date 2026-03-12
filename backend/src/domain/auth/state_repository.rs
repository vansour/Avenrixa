use crate::db::DatabasePool;
use async_trait::async_trait;
use blake3::Hash;
use chrono::{DateTime, Utc};
use sqlx::{MySqlPool, PgPool, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthStateSnapshot {
    pub user_token_version: u64,
    pub session_epoch: u64,
}

#[async_trait]
pub trait AuthStateRepository: Send + Sync {
    async fn load_auth_snapshot(
        &self,
        user_id: Uuid,
    ) -> Result<Option<AuthStateSnapshot>, sqlx::Error>;

    async fn get_user_token_version(&self, user_id: Uuid) -> Result<Option<u64>, sqlx::Error>;

    async fn get_session_epoch(&self) -> Result<u64, sqlx::Error>;

    async fn bump_user_token_version(&self, user_id: Uuid) -> Result<u64, sqlx::Error>;

    async fn bump_session_epoch(&self) -> Result<u64, sqlx::Error>;

    async fn revoke_token_hash(
        &self,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error>;

    async fn is_token_hash_revoked(&self, token_hash: &str) -> Result<bool, sqlx::Error>;

    async fn purge_expired_revoked_tokens(&self, cutoff: DateTime<Utc>)
    -> Result<u64, sqlx::Error>;
}

pub fn hash_token(token: &str) -> String {
    let hash: Hash = blake3::hash(token.as_bytes());
    hash.to_hex().to_string()
}

#[derive(Clone)]
pub struct PostgresAuthStateRepository {
    pool: PgPool,
}

impl PostgresAuthStateRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Clone)]
pub struct MySqlAuthStateRepository {
    pool: MySqlPool,
}

impl MySqlAuthStateRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[derive(Clone)]
pub struct SqliteAuthStateRepository {
    pool: SqlitePool,
}

impl SqliteAuthStateRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(Clone)]
pub enum DatabaseAuthStateRepository {
    Postgres(PostgresAuthStateRepository),
    MySql(MySqlAuthStateRepository),
    Sqlite(SqliteAuthStateRepository),
}

impl DatabaseAuthStateRepository {
    pub fn from_database(database: &DatabasePool) -> Self {
        match database {
            DatabasePool::Postgres(pool) => {
                Self::Postgres(PostgresAuthStateRepository::new(pool.clone()))
            }
            DatabasePool::MySql(pool) => Self::MySql(MySqlAuthStateRepository::new(pool.clone())),
            DatabasePool::Sqlite(pool) => {
                Self::Sqlite(SqliteAuthStateRepository::new(pool.clone()))
            }
        }
    }
}

fn to_u64(value: i64) -> u64 {
    value.max(0) as u64
}

#[async_trait]
impl AuthStateRepository for PostgresAuthStateRepository {
    async fn load_auth_snapshot(
        &self,
        user_id: Uuid,
    ) -> Result<Option<AuthStateSnapshot>, sqlx::Error> {
        let Some(user_token_version) = self.get_user_token_version(user_id).await? else {
            return Ok(None);
        };

        Ok(Some(AuthStateSnapshot {
            user_token_version,
            session_epoch: self.get_session_epoch().await?,
        }))
    }

    async fn get_user_token_version(&self, user_id: Uuid) -> Result<Option<u64>, sqlx::Error> {
        let value = sqlx::query_scalar::<_, i64>(
            "SELECT token_version
             FROM users
             WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(value.map(to_u64))
    }

    async fn get_session_epoch(&self) -> Result<u64, sqlx::Error> {
        let value = sqlx::query_scalar::<_, i64>(
            "SELECT session_epoch
             FROM auth_state
             WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or(0);

        Ok(to_u64(value))
    }

    async fn bump_user_token_version(&self, user_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE users
             SET token_version = token_version + 1
             WHERE id = $1",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        let value = sqlx::query_scalar::<_, i64>(
            "SELECT token_version
             FROM users
             WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(to_u64(value))
    }

    async fn bump_session_epoch(&self) -> Result<u64, sqlx::Error> {
        sqlx::query(
            "INSERT INTO auth_state (id, session_epoch, updated_at)
             VALUES (1, 0, NOW())
             ON CONFLICT (id) DO NOTHING",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "UPDATE auth_state
             SET session_epoch = session_epoch + 1,
                 updated_at = NOW()
             WHERE id = 1",
        )
        .execute(&self.pool)
        .await?;

        let value = sqlx::query_scalar::<_, i64>(
            "SELECT session_epoch
             FROM auth_state
             WHERE id = 1",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(to_u64(value))
    }

    async fn revoke_token_hash(
        &self,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO revoked_tokens (token_hash, expires_at, created_at)
             VALUES ($1, $2, NOW())
             ON CONFLICT (token_hash) DO UPDATE
             SET expires_at = EXCLUDED.expires_at",
        )
        .bind(token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn is_token_hash_revoked(&self, token_hash: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(
                SELECT 1
                FROM revoked_tokens
                WHERE token_hash = $1
                  AND expires_at > $2
            )",
        )
        .bind(token_hash)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await
    }

    async fn purge_expired_revoked_tokens(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM revoked_tokens
             WHERE expires_at <= $1",
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[async_trait]
impl AuthStateRepository for MySqlAuthStateRepository {
    async fn load_auth_snapshot(
        &self,
        user_id: Uuid,
    ) -> Result<Option<AuthStateSnapshot>, sqlx::Error> {
        let Some(user_token_version) = self.get_user_token_version(user_id).await? else {
            return Ok(None);
        };

        Ok(Some(AuthStateSnapshot {
            user_token_version,
            session_epoch: self.get_session_epoch().await?,
        }))
    }

    async fn get_user_token_version(&self, user_id: Uuid) -> Result<Option<u64>, sqlx::Error> {
        let value = sqlx::query_scalar::<_, i64>(
            "SELECT token_version
             FROM users
             WHERE id = ?",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(value.map(to_u64))
    }

    async fn get_session_epoch(&self) -> Result<u64, sqlx::Error> {
        let value = sqlx::query_scalar::<_, i64>(
            "SELECT session_epoch
             FROM auth_state
             WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or(0);

        Ok(to_u64(value))
    }

    async fn bump_user_token_version(&self, user_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE users
             SET token_version = token_version + 1
             WHERE id = ?",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        let value = sqlx::query_scalar::<_, i64>(
            "SELECT token_version
             FROM users
             WHERE id = ?",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(to_u64(value))
    }

    async fn bump_session_epoch(&self) -> Result<u64, sqlx::Error> {
        sqlx::query(
            "INSERT INTO auth_state (id, session_epoch, updated_at)
             VALUES (1, 0, CURRENT_TIMESTAMP(6))
             ON DUPLICATE KEY UPDATE
                 session_epoch = session_epoch",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "UPDATE auth_state
             SET session_epoch = session_epoch + 1,
                 updated_at = CURRENT_TIMESTAMP(6)
             WHERE id = 1",
        )
        .execute(&self.pool)
        .await?;

        let value = sqlx::query_scalar::<_, i64>(
            "SELECT session_epoch
             FROM auth_state
             WHERE id = 1",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(to_u64(value))
    }

    async fn revoke_token_hash(
        &self,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO revoked_tokens (token_hash, expires_at, created_at)
             VALUES (?, ?, CURRENT_TIMESTAMP(6))
             ON DUPLICATE KEY UPDATE
                 expires_at = VALUES(expires_at)",
        )
        .bind(token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn is_token_hash_revoked(&self, token_hash: &str) -> Result<bool, sqlx::Error> {
        let value = sqlx::query_scalar::<_, i64>(
            "SELECT EXISTS(
                SELECT 1
                FROM revoked_tokens
                WHERE token_hash = ?
                  AND expires_at > ?
            )",
        )
        .bind(token_hash)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?;

        Ok(value != 0)
    }

    async fn purge_expired_revoked_tokens(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM revoked_tokens
             WHERE expires_at <= ?",
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[async_trait]
impl AuthStateRepository for SqliteAuthStateRepository {
    async fn load_auth_snapshot(
        &self,
        user_id: Uuid,
    ) -> Result<Option<AuthStateSnapshot>, sqlx::Error> {
        let Some(user_token_version) = self.get_user_token_version(user_id).await? else {
            return Ok(None);
        };

        Ok(Some(AuthStateSnapshot {
            user_token_version,
            session_epoch: self.get_session_epoch().await?,
        }))
    }

    async fn get_user_token_version(&self, user_id: Uuid) -> Result<Option<u64>, sqlx::Error> {
        let value = sqlx::query_scalar::<_, i64>(
            "SELECT token_version
             FROM users
             WHERE id = ?",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(value.map(to_u64))
    }

    async fn get_session_epoch(&self) -> Result<u64, sqlx::Error> {
        let value = sqlx::query_scalar::<_, i64>(
            "SELECT session_epoch
             FROM auth_state
             WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or(0);

        Ok(to_u64(value))
    }

    async fn bump_user_token_version(&self, user_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE users
             SET token_version = token_version + 1
             WHERE id = ?",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        let value = sqlx::query_scalar::<_, i64>(
            "SELECT token_version
             FROM users
             WHERE id = ?",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(to_u64(value))
    }

    async fn bump_session_epoch(&self) -> Result<u64, sqlx::Error> {
        sqlx::query(
            "INSERT OR IGNORE INTO auth_state (id, session_epoch, updated_at)
             VALUES (1, 0, STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "UPDATE auth_state
             SET session_epoch = session_epoch + 1,
                 updated_at = STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = 1",
        )
        .execute(&self.pool)
        .await?;

        let value = sqlx::query_scalar::<_, i64>(
            "SELECT session_epoch
             FROM auth_state
             WHERE id = 1",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(to_u64(value))
    }

    async fn revoke_token_hash(
        &self,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO revoked_tokens (token_hash, expires_at, created_at)
             VALUES (?, ?, STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now'))
             ON CONFLICT(token_hash) DO UPDATE SET
                 expires_at = excluded.expires_at",
        )
        .bind(token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn is_token_hash_revoked(&self, token_hash: &str) -> Result<bool, sqlx::Error> {
        let value = sqlx::query_scalar::<_, i64>(
            "SELECT EXISTS(
                SELECT 1
                FROM revoked_tokens
                WHERE token_hash = ?
                  AND expires_at > ?
            )",
        )
        .bind(token_hash)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?;

        Ok(value != 0)
    }

    async fn purge_expired_revoked_tokens(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM revoked_tokens
             WHERE expires_at <= ?",
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[async_trait]
impl AuthStateRepository for DatabaseAuthStateRepository {
    async fn load_auth_snapshot(
        &self,
        user_id: Uuid,
    ) -> Result<Option<AuthStateSnapshot>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.load_auth_snapshot(user_id).await,
            Self::MySql(repo) => repo.load_auth_snapshot(user_id).await,
            Self::Sqlite(repo) => repo.load_auth_snapshot(user_id).await,
        }
    }

    async fn get_user_token_version(&self, user_id: Uuid) -> Result<Option<u64>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.get_user_token_version(user_id).await,
            Self::MySql(repo) => repo.get_user_token_version(user_id).await,
            Self::Sqlite(repo) => repo.get_user_token_version(user_id).await,
        }
    }

    async fn get_session_epoch(&self) -> Result<u64, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.get_session_epoch().await,
            Self::MySql(repo) => repo.get_session_epoch().await,
            Self::Sqlite(repo) => repo.get_session_epoch().await,
        }
    }

    async fn bump_user_token_version(&self, user_id: Uuid) -> Result<u64, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.bump_user_token_version(user_id).await,
            Self::MySql(repo) => repo.bump_user_token_version(user_id).await,
            Self::Sqlite(repo) => repo.bump_user_token_version(user_id).await,
        }
    }

    async fn bump_session_epoch(&self) -> Result<u64, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.bump_session_epoch().await,
            Self::MySql(repo) => repo.bump_session_epoch().await,
            Self::Sqlite(repo) => repo.bump_session_epoch().await,
        }
    }

    async fn revoke_token_hash(
        &self,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.revoke_token_hash(token_hash, expires_at).await,
            Self::MySql(repo) => repo.revoke_token_hash(token_hash, expires_at).await,
            Self::Sqlite(repo) => repo.revoke_token_hash(token_hash, expires_at).await,
        }
    }

    async fn is_token_hash_revoked(&self, token_hash: &str) -> Result<bool, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.is_token_hash_revoked(token_hash).await,
            Self::MySql(repo) => repo.is_token_hash_revoked(token_hash).await,
            Self::Sqlite(repo) => repo.is_token_hash_revoked(token_hash).await,
        }
    }

    async fn purge_expired_revoked_tokens(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<u64, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.purge_expired_revoked_tokens(cutoff).await,
            Self::MySql(repo) => repo.purge_expired_revoked_tokens(cutoff).await,
            Self::Sqlite(repo) => repo.purge_expired_revoked_tokens(cutoff).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::hash_token;

    #[test]
    fn hash_token_is_stable_hex() {
        let first = hash_token("sample-token");
        let second = hash_token("sample-token");

        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|ch| ch.is_ascii_hexdigit()));
    }
}
