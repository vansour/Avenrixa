/// 图片缓存键生成器
#[allow(dead_code)]
pub struct ImageCache;

impl ImageCache {
    pub fn list(
        user_id: uuid::Uuid,
        page: i32,
        page_size: i32,
        category_id: Option<uuid::Uuid>,
    ) -> String {
        match category_id {
            Some(category_id) => format!(
                "images:list:{}:{}:{}:{}",
                user_id, category_id, page, page_size
            ),
            None => format!("images:list:{}:{}:{}", user_id, page, page_size),
        }
    }

    #[allow(dead_code)]
    pub fn categories(user_id: uuid::Uuid) -> String {
        format!("categories:list:{}", user_id)
    }

    pub fn categories_invalidate(user_id: uuid::Uuid) -> String {
        format!("categories:list:{}*", user_id)
    }

    pub fn images_invalidate(user_id: uuid::Uuid) -> String {
        format!("images:list:{}:*", user_id)
    }
}
