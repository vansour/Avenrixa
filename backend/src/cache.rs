use redis::AsyncCommands;
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

    /// 获取缓存值
    pub async fn get<T, C>(conn: &mut C, key: impl AsRef<str>) -> Result<Option<T>, anyhow::Error>
    where
        T: DeserializeOwned,
        C: redis::aio::ConnectionLike + Send + Sync,
    {
        // 如果连接处于某种不可用状态，我们可以直接返回 None 而不是 Error
        // 这样在测试中如果没有 Redis，业务逻辑仍然可以继续运行（只是没有缓存）
        let cache = Self::new("");
        let key = cache.key(key);
        let value: Result<Option<String>, _> = conn.get(&key).await;

        match value {
            Ok(Some(v)) => serde_json::from_str(&v)
                .map_err(|e| anyhow::anyhow!("Deserialization failed: {}", e))
                .map(Some),
            Ok(None) => Ok(None),
            Err(e) => {
                warn!("Redis get error (key: {}): {}", key, e);
                Ok(None)
            }
        }
    }

    /// 设置缓存值
    pub async fn set<C>(
        conn: &mut C,
        key: impl AsRef<str>,
        value: impl Serialize,
        ttl_seconds: u64,
    ) -> Result<(), anyhow::Error>
    where
        C: redis::aio::ConnectionLike + Send + Sync,
    {
        let cache = Self::new("");
        let key = cache.key(key);
        let value = serde_json::to_string(&value)
            .map_err(|e| anyhow::anyhow!("Serialization failed: {}", e))?;
        let result: Result<(), _> = conn.set_ex(&key, value, ttl_seconds).await;

        if let Err(e) = result {
            warn!("Redis set error (key: {}): {}", key, e);
        }

        Ok(())
    }

    /// 删除匹配前缀的所有缓存（使用 SCAN 替代 KEYS 避免阻塞）
    #[allow(dead_code)]
    pub async fn del_pattern<C>(conn: &mut C, pattern: impl AsRef<str>) -> Result<(), anyhow::Error>
    where
        C: redis::aio::ConnectionLike + Send + Sync,
    {
        let cache = Self::new("");
        let pattern = cache.key(pattern);
        let mut cursor: u64 = 0;

        loop {
            // 使用 SCAN 迭代，每次获取 100 个key
            let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(conn)
                .await
                .map_err(|e| anyhow::anyhow!("Redis SCAN error: {}", e))?;

            if !keys.is_empty() {
                conn.del::<_, ()>(keys)
                    .await
                    .map_err(|e| anyhow::anyhow!("Redis DEL error: {}", e))?;
            }

            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }

        Ok(())
    }
}

/// 图片 hash 缓存键生成器
#[allow(dead_code)]
pub struct HashCache;

#[allow(dead_code)]
impl HashCache {
    pub fn image_hash(hash: &str, strategy: &str) -> String {
        match strategy {
            "global" => format!("hash:global:{}", hash),
            _ => format!("hash:user:{}", hash),
        }
    }

    pub fn existing_info(hash: &str, strategy: &str, user_id: uuid::Uuid) -> String {
        match strategy {
            "global" => format!("hash:info:global:{}", hash),
            _ => format!("hash:info:user:{}:{}", hash, user_id),
        }
    }
}

/// 图片缓存键生成器
#[allow(dead_code)]
pub struct ImageCache;

#[allow(dead_code)]
impl ImageCache {
    pub fn list(
        user_id: uuid::Uuid,
        page: i32,
        page_size: i32,
        category_id: Option<uuid::Uuid>,
        sort_by: &str,
        sort_order: &str,
    ) -> String {
        match category_id {
            Some(cat_id) => format!(
                "images:list:{}:{}:{}:{}:{}:{}",
                user_id, cat_id, page, page_size, sort_by, sort_order
            ),
            None => format!(
                "images:list:{}:{}:{}:{}:{}",
                user_id, page, page_size, sort_by, sort_order
            ),
        }
    }

    pub fn categories(user_id: uuid::Uuid) -> String {
        format!("categories:list:{}", user_id)
    }

    /// 用户分类列表失效模式
    pub fn categories_invalidate(user_id: uuid::Uuid) -> String {
        // '*' 可匹配空后缀，兼容精确 key 与未来扩展 key
        format!("categories:list:{}*", user_id)
    }

    /// 用户图片列表失效模式
    pub fn images_invalidate(user_id: uuid::Uuid) -> String {
        format!("images:list:{}:*", user_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_image_cache_list_without_category() {
        let user_id = Uuid::new_v4();
        let page = 1;
        let page_size = 20;
        let sort_by = "created_at";
        let sort_order = "DESC";

        let key = ImageCache::list(user_id, page, page_size, None, sort_by, sort_order);

        // 格式: images:list:{user_id}:{page}:{page_size}:{sort_by}:{sort_order}
        assert!(key.contains(&user_id.to_string()));
        assert!(key.contains(&page.to_string()));
        assert!(key.contains(&page_size.to_string()));
        assert!(key.contains(sort_by));
        assert!(key.contains(sort_order));
        assert!(key.starts_with("images:list:"));
    }

    #[test]
    fn test_image_cache_list_with_category() {
        let user_id = Uuid::new_v4();
        let page = 2;
        let page_size = 50;
        let category_id = Some(Uuid::new_v4());
        let sort_by = "size";
        let sort_order = "ASC";

        let key = ImageCache::list(user_id, page, page_size, category_id, sort_by, sort_order);

        // 格式: images:list:{user_id}:{category_id}:{page}:{page_size}:{sort_by}:{sort_order}
        assert!(key.contains(&user_id.to_string()));
        if let Some(cat_id) = category_id {
            assert!(key.contains(&cat_id.to_string()));
        }
        assert!(key.contains(&page.to_string()));
        assert!(key.contains(&page_size.to_string()));
        assert!(key.contains(sort_by));
        assert!(key.contains(sort_order));
    }

    #[test]
    fn test_image_cache_categories() {
        let user_id = Uuid::new_v4();
        let key = ImageCache::categories(user_id);
        assert_eq!(key, format!("categories:list:{}", user_id));
    }

    #[test]
    fn test_cache_key_prefix() {
        let cache = Cache::new("test_prefix");
        let key = cache.key("test_key");
        assert_eq!(key, "test_prefixtest_key");
    }

    #[test]
    fn test_cache_key_empty_prefix() {
        let cache = Cache::new("");
        let key = cache.key("test_key");
        assert_eq!(key, "test_key");
    }

    #[test]
    fn test_image_cache_different_sort_orders() {
        let user_id = Uuid::new_v4();

        let key1 = ImageCache::list(user_id, 1, 20, None, "created_at", "DESC");
        let key2 = ImageCache::list(user_id, 1, 20, None, "created_at", "ASC");
        let key3 = ImageCache::list(user_id, 1, 20, None, "size", "DESC");

        assert_ne!(key1, key2); // 排序方向不同，键应不同
        assert_ne!(key1, key3); // 排序字段不同，键应不同
        assert_ne!(key2, key3);
    }
}
