/// 图片 hash 缓存键生成器
#[allow(dead_code)]
pub struct HashCache;

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

    pub fn user_existing_info_invalidate(user_id: uuid::Uuid) -> String {
        format!("hash:info:user:*:{}", user_id)
    }

    pub fn user_hash_invalidate() -> String {
        "hash:user:*".to_string()
    }

    pub fn global_existing_info_invalidate() -> String {
        "hash:info:global:*".to_string()
    }

    pub fn global_hash_invalidate() -> String {
        "hash:global:*".to_string()
    }
}
