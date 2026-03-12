use super::{CacheCommands, CacheConnectionLike};
use serde::{Serialize, de::DeserializeOwned};
use tracing::warn;

/// 缓存辅助工具
pub struct Cache {
    key_prefix: String,
}

impl Cache {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            key_prefix: prefix.into(),
        }
    }

    pub fn key(&self, key: impl AsRef<str>) -> String {
        format!("{}{}", self.key_prefix, key.as_ref())
    }

    pub async fn get<T, C>(conn: &mut C, key: impl AsRef<str>) -> Result<Option<T>, anyhow::Error>
    where
        T: DeserializeOwned,
        C: CacheConnectionLike + Send + Sync,
    {
        let key = prefixed_key(key);
        let value: Result<Option<String>, _> = conn.get(&key).await;

        match value {
            Ok(Some(value)) => serde_json::from_str(&value)
                .map_err(|error| anyhow::anyhow!("Deserialization failed: {}", error))
                .map(Some),
            Ok(None) => Ok(None),
            Err(error) => {
                warn!("External cache get error (key: {}): {}", key, error);
                Ok(None)
            }
        }
    }

    pub async fn set<C>(
        conn: &mut C,
        key: impl AsRef<str>,
        value: impl Serialize,
        ttl_seconds: u64,
    ) -> Result<(), anyhow::Error>
    where
        C: CacheConnectionLike + Send + Sync,
    {
        let key = prefixed_key(key);
        let value = serde_json::to_string(&value)
            .map_err(|error| anyhow::anyhow!("Serialization failed: {}", error))?;
        let result: Result<(), _> = conn.set_ex(&key, value, ttl_seconds).await;

        if let Err(error) = result {
            warn!("External cache set error (key: {}): {}", key, error);
        }

        Ok(())
    }

    pub async fn del<C>(conn: &mut C, key: impl AsRef<str>) -> Result<(), anyhow::Error>
    where
        C: CacheConnectionLike + Send + Sync,
    {
        let key = prefixed_key(key);
        let result: Result<(), _> = conn.del(&key).await;
        if let Err(error) = result {
            warn!("External cache del error (key: {}): {}", key, error);
        }
        Ok(())
    }

    pub async fn del_pattern<C>(conn: &mut C, pattern: impl AsRef<str>) -> Result<(), anyhow::Error>
    where
        C: CacheConnectionLike + Send + Sync,
    {
        let pattern = prefixed_key(pattern);
        let mut cursor: u64 = 0;

        loop {
            let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(conn)
                .await
                .map_err(|error| anyhow::anyhow!("External cache SCAN error: {}", error))?;

            if !keys.is_empty() {
                conn.del::<_, ()>(keys)
                    .await
                    .map_err(|error| anyhow::anyhow!("External cache DEL error: {}", error))?;
            }

            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }

        Ok(())
    }
}

fn prefixed_key(key: impl AsRef<str>) -> String {
    Cache::new("").key(key)
}
