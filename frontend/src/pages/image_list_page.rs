use crate::app_context::{use_image_service, use_image_store, use_toast_store};
use crate::components::{ImageGrid, Loading};
use crate::types::api::PaginationParams;
use crate::types::models::ImageItem;
use dioxus::prelude::*;
use std::collections::HashSet;
use uuid::Uuid;

#[cfg(target_arch = "wasm32")]
fn open_in_new_tab(url: &str) -> bool {
    web_sys::window()
        .and_then(|window| window.open_with_url_and_target(url, "_blank").ok())
        .is_some()
}

#[cfg(not(target_arch = "wasm32"))]
fn open_in_new_tab(_url: &str) -> bool {
    false
}

/// 图片列表页面组件
#[component]
pub fn ImageListPage() -> Element {
    let image_service = use_image_service();
    let image_store = use_image_store();
    let toast_store = use_toast_store();

    // 加载状态
    let mut is_loading = use_signal(|| false);
    let mut is_deleting = use_signal(|| false);
    let mut error_message = use_signal(String::new);
    let mut selected_ids = use_signal(HashSet::<Uuid>::new);
    let mut reload_tick = use_signal(|| 0_u64);
    let mut current_page = use_signal(|| 1_i32);

    // 页面加载时获取图片数据，并支持手动刷新重跑
    let _load_images = use_resource({
        let image_service = image_service.clone();
        let toast_store = toast_store.clone();
        move || {
            let _ = reload_tick();
            let page = current_page().max(1);
            let image_service = image_service.clone();
            let toast_store = toast_store.clone();
            async move {
                is_loading.set(true);
                error_message.set(String::new());

                let params = PaginationParams {
                    page: Some(page),
                    page_size: Some(20),
                    sort_by: "created_at".to_string(),
                    sort_order: "DESC".to_string(),
                    search: None,
                    category_id: None,
                    tag: None,
                    cursor: None,
                };

                match image_service.get_images(params).await {
                    Ok(result) => {
                        // 删除后可能出现页码越界：自动回退到上一页重拉
                        if result.data.is_empty() && page > 1 && result.total > 0 {
                            current_page.set(page - 1);
                            is_loading.set(false);
                            return;
                        }

                        // 列表与分页元数据已由 ImageService 同步到 Store
                        current_page.set(result.page.max(1));
                        selected_ids.set(HashSet::new());
                    }
                    Err(e) => {
                        let message = format!("加载图片失败: {}", e);
                        error_message.set(message.clone());
                        toast_store.show_error(message);
                    }
                }

                is_loading.set(false);
            }
        }
    });

    let handle_refresh = move |_| {
        reload_tick.set(reload_tick().wrapping_add(1));
    };

    let handle_prev_page = move |_| {
        if is_loading() || current_page() <= 1 {
            return;
        }
        current_page.set(current_page() - 1);
    };

    let image_store_for_next_page = image_store.clone();
    let handle_next_page = move |_| {
        if is_loading() || !image_store_for_next_page.has_more() {
            return;
        }
        current_page.set(current_page() + 1);
    };

    let handle_toggle_select = move |image_id: Uuid| {
        let mut ids = selected_ids();
        if !ids.insert(image_id) {
            ids.remove(&image_id);
        }
        selected_ids.set(ids);
    };

    let toast_store_for_download = toast_store.clone();
    let handle_download = move |image: ImageItem| {
        let url = image.url();
        if !open_in_new_tab(&url) {
            toast_store_for_download.show_info(format!("图片地址: {}", url));
        }
    };

    let image_service_for_delete = image_service.clone();
    let toast_store_for_delete = toast_store.clone();
    let handle_delete = move |image: ImageItem| {
        if is_deleting() {
            return;
        }

        let image_service = image_service_for_delete.clone();
        let toast_store = toast_store_for_delete.clone();
        spawn(async move {
            is_deleting.set(true);

            match image_service.delete_images(vec![image.id], false).await {
                Ok(_) => {
                    toast_store.show_success(format!("已删除: {}", image.original_filename()));
                    let mut ids = selected_ids();
                    ids.remove(&image.id);
                    selected_ids.set(ids);
                    reload_tick.set(reload_tick().wrapping_add(1));
                }
                Err(e) => {
                    toast_store.show_error(format!("删除失败: {}", e));
                }
            }

            is_deleting.set(false);
        });
    };

    let image_service_for_batch_delete = image_service.clone();
    let toast_store_for_batch_delete = toast_store.clone();
    let handle_delete_selected = move |_| {
        if is_deleting() {
            return;
        }

        let ids = selected_ids();
        if ids.is_empty() {
            return;
        }

        let delete_list: Vec<Uuid> = ids.iter().cloned().collect();
        let count = delete_list.len();
        let image_service = image_service_for_batch_delete.clone();
        let toast_store = toast_store_for_batch_delete.clone();

        spawn(async move {
            is_deleting.set(true);

            match image_service.delete_images(delete_list, false).await {
                Ok(_) => {
                    toast_store.show_success(format!("已删除 {} 张图片", count));
                    selected_ids.set(HashSet::new());
                    reload_tick.set(reload_tick().wrapping_add(1));
                }
                Err(e) => {
                    toast_store.show_error(format!("批量删除失败: {}", e));
                }
            }

            is_deleting.set(false);
        });
    };

    rsx! {
        div { class: "image-list-page",
            header { class: "page-header",
                h1 { "我的图片" }
                div { class: "page-actions",
                    if !selected_ids().is_empty() {
                        button {
                            class: "btn btn-danger",
                            disabled: is_loading() || is_deleting(),
                            onclick: handle_delete_selected,
                            "删除所选 ({selected_ids().len()})"
                        }
                    }

                    button {
                        class: "btn btn-primary",
                        disabled: is_loading() || is_deleting(),
                        onclick: handle_refresh,
                        if is_loading() {
                            "刷新中..."
                        } else {
                            "刷新"
                        }
                    }
                }
            }

            div { class: "pagination-bar",
                span { class: "page-summary",
                    "第 {current_page()} 页 · 共 {image_store.total_items()} 项"
                }
                div { class: "page-actions",
                    button {
                        class: "btn",
                        disabled: is_loading() || is_deleting() || current_page() <= 1,
                        onclick: handle_prev_page,
                        "上一页"
                    }
                    button {
                        class: "btn",
                        disabled: is_loading() || is_deleting() || !image_store.has_more(),
                        onclick: handle_next_page,
                        "下一页"
                    }
                }
            }

            if !error_message().is_empty() {
                div { class: "error-message",
                    "{error_message()}"
                }
            }

            div { class: "image-list-wrapper",
                if is_loading() {
                    Loading {}
                } else if image_store.images_is_empty() {
                    div { class: "empty-state",
                        p { "暂无图片" }
                    }
                } else {
                    ImageGrid {
                        images: image_store.images(),
                        selected_ids: selected_ids(),
                        on_toggle_select: handle_toggle_select,
                        on_download: handle_download,
                        on_delete: handle_delete,
                    }
                }
            }
        }
    }
}
