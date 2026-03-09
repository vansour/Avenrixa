use crate::types::models::ImageItem;
use dioxus::prelude::*;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Clone, Default)]
struct ImageState {
    images: Vec<ImageItem>,
    current_page: u32,
    total_items: u64,
    selected_ids: HashSet<String>,
    is_loading: bool,
    has_more: bool,
}

/// 图片状态管理 Store
#[derive(Clone)]
pub struct ImageStore {
    state: Rc<RefCell<Signal<ImageState>>>,
}

impl ImageStore {
    pub fn new() -> Self {
        let initial_state = ImageState {
            images: Vec::new(),
            current_page: 1,
            total_items: 0,
            selected_ids: HashSet::new(),
            is_loading: false,
            has_more: true,
        };

        Self {
            state: Rc::new(RefCell::new(Signal::new(initial_state))),
        }
    }

    /// 获取图片列表
    pub fn images(&self) -> Vec<ImageItem> {
        self.state.borrow().read().images.clone()
    }

    /// 检查图片列表是否为空
    pub fn images_is_empty(&self) -> bool {
        self.state.borrow().read().images.is_empty()
    }

    /// 获取当前页码
    pub fn current_page(&self) -> u32 {
        self.state.borrow().read().current_page
    }

    /// 获取总项目数
    pub fn total_items(&self) -> u64 {
        self.state.borrow().read().total_items
    }

    /// 添加图片
    pub fn add_images(&self, new_images: Vec<ImageItem>) {
        self.state.borrow_mut().write().images.extend(new_images);
    }

    /// 设置加载状态
    pub fn set_loading(&self, loading: bool) {
        self.state.borrow_mut().write().is_loading = loading;
    }

    /// 获取加载状态
    pub fn is_loading(&self) -> bool {
        self.state.borrow().read().is_loading
    }

    /// 增加页码
    pub fn increment_page(&self) {
        self.state.borrow_mut().write().current_page += 1;
    }

    /// 设置当前页码
    pub fn set_current_page(&self, page: u32) {
        self.state.borrow_mut().write().current_page = page.max(1);
    }

    /// 设置是否有更多
    pub fn set_has_more(&self, more: bool) {
        self.state.borrow_mut().write().has_more = more;
    }

    /// 获取是否有更多
    pub fn has_more(&self) -> bool {
        self.state.borrow().read().has_more
    }

    /// 清空图片列表
    pub fn clear_images(&self) {
        let mut state_signal = self.state.borrow_mut();
        let mut state = state_signal.write();
        state.images.clear();
        state.current_page = 1;
    }

    /// 设置图片列表
    pub fn set_images(&self, new_images: Vec<ImageItem>) {
        self.state.borrow_mut().write().images = new_images;
    }

    /// 设置分页元数据
    pub fn set_pagination(&self, page: u32, total_items: u64, has_more: bool) {
        let mut state_signal = self.state.borrow_mut();
        let mut state = state_signal.write();
        state.current_page = page.max(1);
        state.total_items = total_items;
        state.has_more = has_more;
    }

    #[allow(dead_code)]
    pub fn selected_ids(&self) -> HashSet<String> {
        self.state.borrow().read().selected_ids.clone()
    }
}

impl Default for ImageStore {
    fn default() -> Self {
        Self::new()
    }
}
