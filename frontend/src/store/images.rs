use crate::types::models::ImageItem;
use dioxus::prelude::*;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageCollectionKind {
    Active,
    Deleted,
}

#[derive(Clone, Default)]
pub struct ImageCollectionSnapshot {
    pub images: Vec<ImageItem>,
    pub current_page: u32,
    pub page_size: u32,
    pub total_items: u64,
    pub selected_ids: HashSet<String>,
    pub is_loading: bool,
    pub is_processing: bool,
    pub has_more: bool,
    pub error_message: String,
    pub reload_token: u64,
}

#[derive(Clone, Default)]
struct ImageCollectionsState {
    active: ImageCollectionSnapshot,
    deleted: ImageCollectionSnapshot,
}

impl ImageCollectionsState {
    fn collection(&self, kind: ImageCollectionKind) -> &ImageCollectionSnapshot {
        match kind {
            ImageCollectionKind::Active => &self.active,
            ImageCollectionKind::Deleted => &self.deleted,
        }
    }

    fn collection_mut(&mut self, kind: ImageCollectionKind) -> &mut ImageCollectionSnapshot {
        match kind {
            ImageCollectionKind::Active => &mut self.active,
            ImageCollectionKind::Deleted => &mut self.deleted,
        }
    }
}

/// 图片状态管理 Store
#[derive(Clone)]
pub struct ImageStore {
    state: Rc<RefCell<Signal<ImageCollectionsState>>>,
}

impl ImageStore {
    pub fn new() -> Self {
        let mut initial_state = ImageCollectionsState::default();
        initial_state.active.current_page = 1;
        initial_state.active.page_size = 20;
        initial_state.active.has_more = true;
        initial_state.deleted.current_page = 1;
        initial_state.deleted.page_size = 20;
        initial_state.deleted.has_more = true;

        Self {
            state: Rc::new(RefCell::new(Signal::new(initial_state))),
        }
    }

    pub fn collection(&self, kind: ImageCollectionKind) -> ImageCollectionSnapshot {
        self.state.borrow().read().collection(kind).clone()
    }

    pub fn set_loading(&self, kind: ImageCollectionKind, is_loading: bool) {
        self.state
            .borrow_mut()
            .write()
            .collection_mut(kind)
            .is_loading = is_loading;
    }

    pub fn set_processing(&self, kind: ImageCollectionKind, is_processing: bool) {
        self.state
            .borrow_mut()
            .write()
            .collection_mut(kind)
            .is_processing = is_processing;
    }

    pub fn set_error_message(&self, kind: ImageCollectionKind, message: impl Into<String>) {
        self.state
            .borrow_mut()
            .write()
            .collection_mut(kind)
            .error_message = message.into();
    }

    pub fn clear_error(&self, kind: ImageCollectionKind) {
        self.set_error_message(kind, String::new());
    }

    pub fn set_page(&self, kind: ImageCollectionKind, page: u32) {
        self.state
            .borrow_mut()
            .write()
            .collection_mut(kind)
            .current_page = page.max(1);
    }

    pub fn set_page_size(&self, kind: ImageCollectionKind, page_size: u32) {
        self.state
            .borrow_mut()
            .write()
            .collection_mut(kind)
            .page_size = page_size.clamp(1, 100);
    }

    pub fn replace_page(
        &self,
        kind: ImageCollectionKind,
        images: Vec<ImageItem>,
        current_page: u32,
        page_size: u32,
        total_items: u64,
        has_more: bool,
    ) {
        let mut state = self.state.borrow_mut();
        let mut state = state.write();
        let collection = state.collection_mut(kind);
        collection.images = images;
        collection.current_page = current_page.max(1);
        collection.page_size = page_size.clamp(1, 100);
        collection.total_items = total_items;
        collection.has_more = has_more;
        collection.selected_ids.clear();
    }

    pub fn toggle_selection(&self, kind: ImageCollectionKind, image_key: &str) {
        let mut state = self.state.borrow_mut();
        let mut state = state.write();
        let selected_ids = &mut state.collection_mut(kind).selected_ids;
        if !selected_ids.insert(image_key.to_string()) {
            selected_ids.remove(image_key);
        }
    }

    pub fn clear_selection(&self, kind: ImageCollectionKind) {
        self.state
            .borrow_mut()
            .write()
            .collection_mut(kind)
            .selected_ids
            .clear();
    }

    pub fn remove_selection(&self, kind: ImageCollectionKind, image_key: &str) {
        self.state
            .borrow_mut()
            .write()
            .collection_mut(kind)
            .selected_ids
            .remove(image_key);
    }

    pub fn toggle_all_visible(&self, kind: ImageCollectionKind) {
        let mut state = self.state.borrow_mut();
        let mut state = state.write();
        let collection = state.collection_mut(kind);
        let is_all_selected = !collection.images.is_empty()
            && collection
                .images
                .iter()
                .all(|image| collection.selected_ids.contains(&image.image_key));

        if is_all_selected {
            for image in &collection.images {
                collection.selected_ids.remove(&image.image_key);
            }
        } else {
            for image in &collection.images {
                collection.selected_ids.insert(image.image_key.clone());
            }
        }
    }

    pub fn mark_for_reload(&self, kind: ImageCollectionKind) {
        self.state
            .borrow_mut()
            .write()
            .collection_mut(kind)
            .reload_token += 1;
    }
}

impl Default for ImageStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use dioxus::prelude::{rsx, ScopeId, VirtualDom};

    struct TestImageStoreHarness {
        _dom: VirtualDom,
        store: ImageStore,
    }

    impl TestImageStoreHarness {
        fn new() -> Self {
            let dom = VirtualDom::new(|| rsx! {});
            let store = dom.in_scope(ScopeId::ROOT, ImageStore::new);
            Self { _dom: dom, store }
        }
    }

    fn sample_image(image_key: &str, filename: &str) -> ImageItem {
        ImageItem {
            image_key: image_key.to_string(),
            filename: filename.to_string(),
            size: 1024,
            format: "png".to_string(),
            views: 0,
            status: "active".to_string(),
            expires_at: None,
            deleted_at: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn new_store_initializes_pagination_defaults() {
        let harness = TestImageStoreHarness::new();
        let store = &harness.store;

        let active = store.collection(ImageCollectionKind::Active);
        let deleted = store.collection(ImageCollectionKind::Deleted);

        assert_eq!(active.current_page, 1);
        assert_eq!(active.page_size, 20);
        assert!(active.has_more);
        assert_eq!(deleted.current_page, 1);
        assert_eq!(deleted.page_size, 20);
        assert!(deleted.has_more);
    }

    #[test]
    fn replace_page_clamps_pagination_and_clears_selection() {
        let harness = TestImageStoreHarness::new();
        let store = &harness.store;
        store.toggle_selection(ImageCollectionKind::Active, "first");

        store.replace_page(
            ImageCollectionKind::Active,
            vec![sample_image("first", "first.png")],
            0,
            500,
            12,
            false,
        );

        let snapshot = store.collection(ImageCollectionKind::Active);
        assert_eq!(snapshot.current_page, 1);
        assert_eq!(snapshot.page_size, 100);
        assert_eq!(snapshot.total_items, 12);
        assert!(!snapshot.has_more);
        assert!(snapshot.selected_ids.is_empty());
    }

    #[test]
    fn toggle_all_visible_selects_then_clears_current_page() {
        let harness = TestImageStoreHarness::new();
        let store = &harness.store;
        store.replace_page(
            ImageCollectionKind::Deleted,
            vec![
                sample_image("first", "first.png"),
                sample_image("second", "second.png"),
            ],
            1,
            20,
            2,
            false,
        );
        store.toggle_selection(ImageCollectionKind::Deleted, "off-page");

        store.toggle_all_visible(ImageCollectionKind::Deleted);
        let selected_once = store.collection(ImageCollectionKind::Deleted);
        assert_eq!(selected_once.selected_ids.len(), 3);
        assert!(selected_once.selected_ids.contains("first"));
        assert!(selected_once.selected_ids.contains("second"));
        assert!(selected_once.selected_ids.contains("off-page"));

        store.toggle_all_visible(ImageCollectionKind::Deleted);
        let selected_twice = store.collection(ImageCollectionKind::Deleted);
        assert_eq!(selected_twice.selected_ids.len(), 1);
        assert!(selected_twice.selected_ids.contains("off-page"));
    }

    #[test]
    fn status_helpers_update_collection_state() {
        let harness = TestImageStoreHarness::new();
        let store = &harness.store;

        store.set_loading(ImageCollectionKind::Active, true);
        store.set_processing(ImageCollectionKind::Active, true);
        store.set_error_message(ImageCollectionKind::Active, "boom");
        store.mark_for_reload(ImageCollectionKind::Active);
        store.mark_for_reload(ImageCollectionKind::Active);
        store.set_page(ImageCollectionKind::Active, 0);
        store.set_page_size(ImageCollectionKind::Active, 0);
        store.clear_error(ImageCollectionKind::Active);

        let snapshot = store.collection(ImageCollectionKind::Active);
        assert!(snapshot.is_loading);
        assert!(snapshot.is_processing);
        assert_eq!(snapshot.current_page, 1);
        assert_eq!(snapshot.page_size, 1);
        assert_eq!(snapshot.reload_token, 2);
        assert!(snapshot.error_message.is_empty());
    }
}
