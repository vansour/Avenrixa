use super::*;

impl<I: ImageRepository, C: CategoryRepository> ImageDomainService<I, C> {
    /// 获取分类列表
    pub async fn get_categories(&self, user_id: Uuid) -> Result<Vec<Category>, AppError> {
        let categories = self
            .category_repository
            .find_categories_by_user(user_id)
            .await?;
        Ok(categories)
    }

    /// 创建分类
    pub async fn create_category(&self, category: &Category) -> Result<(), AppError> {
        self.category_repository.create_category(category).await?;
        Ok(())
    }

    /// 删除分类
    pub async fn delete_category(&self, id: Uuid) -> Result<(), AppError> {
        self.category_repository.delete_category(id).await?;
        Ok(())
    }

    /// 获取配置引用
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 获取图片处理器引用
    /// 获取图片处理器引用
    pub fn image_processor(&self) -> &ImageProcessor {
        &self.image_processor
    }

    /// 获取 Redis 连接引用
    pub fn redis(&self) -> Option<redis::aio::ConnectionManager> {
        self.redis.clone()
    }
}
