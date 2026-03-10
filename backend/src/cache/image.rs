/// 图片缓存键生成器
pub struct ImageCache;

impl ImageCache {
    pub fn list(user_id: uuid::Uuid, page: i32, page_size: i32) -> String {
        format!("images:list:{}:{}:{}", user_id, page, page_size)
    }

    pub fn images_invalidate(user_id: uuid::Uuid) -> String {
        format!("images:list:{}:*", user_id)
    }
}
