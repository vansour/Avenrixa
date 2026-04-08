/// 图片 hash 缓存键生成器
pub struct HashCache;

impl HashCache {
    pub fn existing_info(hash: &str, strategy: &str, user_id: uuid::Uuid) -> String {
        match strategy {
            "global" => format!("hash:info:global:{}", hash),
            _ => format!("hash:info:user:{}:{}", hash, user_id),
        }
    }
}
