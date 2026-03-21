/// 图片缓存键生成器
pub struct ImageCache;

impl ImageCache {
    pub fn list(user_id: uuid::Uuid, cursor: Option<&str>, limit: i32) -> String {
        let cursor = cursor.unwrap_or("first");
        format!("images:list:{}:{}:{}", user_id, cursor, limit)
    }

    pub fn images_invalidate(user_id: uuid::Uuid) -> String {
        format!("images:list:{}:*", user_id)
    }
}
