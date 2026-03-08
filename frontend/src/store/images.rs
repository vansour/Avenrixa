use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashSet;
use crate::types::models::{ImageItem, ImageFilters};

/// 图片状态管理 Store
#[derive(Clone)]
pub struct ImageStore {
    images: Arc<RwLock<Vec<ImageItem>>>,
    current_page: Arc<RwLock<u32>>,
    total_items: Arc<RwLock<u64>>,
    filters: Arc<RwLock<ImageFilters>>,
    selected_ids: Arc<RwLock<HashSet<String>>>,
    is_loading: Arc<RwLock<bool>>,
    has_more: Arc<RwLock<bool>>,
}

impl ImageStore {
    pub fn new() -> Self {
        Self {
            images: Arc::new(RwLock::new(Vec::new())),
            current_page: Arc::new(RwLock::new(1)),
            total_items: Arc::new(RwLock::new(0)),
            filters: Arc::new(RwLock::new(ImageFilters::default())),
            selected_ids: Arc::new(RwLock::new(HashSet::new())),
            is_loading: Arc::new(RwLock::new(false)),
            has_more: Arc::new(RwLock::new(true)),
        }
    }

    /// 获取图片列表
    pub fn images(&self) -> Vec<ImageItem> {
        self.images.read().clone()
    }

    /// 检查图片列表是否为空
    pub fn images_is_empty(&self) -> bool {
        self.images.read().is_empty()
    }

    /// 获取当前页码
    pub fn current_page(&self) -> u32 {
        *self.current_page.read()
    }

    /// 获取总项目数
    pub fn total_items(&self) -> u64 {
        *self.total_items.read()
    }

    /// 获取过滤器
    pub fn filters(&self) -> ImageFilters {
        self.filters.read().clone()
    }

    /// 添加图片
    pub fn add_images(&self, new_images: Vec<ImageItem>) {
        self.images.write().extend(new_images);
    }

    /// 设置加载状态
    pub fn set_loading(&self, loading: bool) {
        *self.is_loading.write() = loading;
    }

    /// 获取加载状态
    pub fn is_loading(&self) -> bool {
        *self.is_loading.read()
    }

    /// 增加页码
    pub fn increment_page(&self) {
        *self.current_page.write() += 1;
    }

    /// 设置是否有更多
    pub fn set_has_more(&self, more: bool) {
        *self.has_more.write() = more;
    }

    /// 获取是否有更多
    pub fn has_more(&self) -> bool {
        *self.has_more.read()
    }

    /// 设置过滤器
    pub fn set_filters(&self, filters: ImageFilters) {
        *self.filters.write() = filters;
    }

    /// 清空图片列表
    pub fn clear_images(&self) {
        self.images.write().clear();
        *self.current_page.write() = 1;
    }
}

impl Default for ImageStore {
    fn default() -> Self {
        Self::new()
    }
}
