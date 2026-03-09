use crate::app_context::{use_image_service, use_image_store, use_toast_store};
use crate::components::{ImageGrid, Loading};
use crate::types::api::PaginationParams;
use crate::types::models::ImageItem;
use dioxus::prelude::*;
use std::collections::HashSet;

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

    let mut is_loading = use_signal(|| false);
    let mut is_deleting = use_signal(|| false);
    let mut error_message = use_signal(String::new);
    let mut selected_ids = use_signal(HashSet::<String>::new);
    let mut reload_tick = use_signal(|| 0_u64);
    let mut current_page = use_signal(|| 1_i32);
    let mut page_size = use_signal(|| 20_i32);

    let _load_images = use_resource({
        let image_service = image_service.clone();
        let toast_store = toast_store.clone();
        move || {
            let _ = reload_tick();
            let page = current_page().max(1);
            let size = page_size().clamp(1, 100);
            let image_service = image_service.clone();
            let toast_store = toast_store.clone();
            async move {
                is_loading.set(true);
                error_message.set(String::new());

                let params = PaginationParams {
                    page: Some(page),
                    page_size: Some(size),
                };

                match image_service.get_images(params).await {
                    Ok(result) => {
                        if result.data.is_empty() && page > 1 && result.total > 0 {
                            current_page.set(page - 1);
                            is_loading.set(false);
                            return;
                        }

                        let normalized_page = result.page.max(1);
                        if normalized_page != page {
                            current_page.set(normalized_page);
                        }
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

    let handle_toggle_select = move |image_key: String| {
        let mut ids = selected_ids();
        if !ids.insert(image_key.clone()) {
            ids.remove(&image_key);
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

            match image_service
                .delete_images(vec![image.image_key.clone()], false)
                .await
            {
                Ok(_) => {
                    toast_store.show_success(format!("已删除: {}", image.display_name()));
                    let mut ids = selected_ids();
                    ids.remove(&image.image_key);
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

    let image_store_for_toggle_all = image_store.clone();
    let handle_toggle_all = move |_| {
        let images = image_store_for_toggle_all.images();
        if images.is_empty() {
            selected_ids.set(HashSet::new());
            return;
        }

        let mut ids = selected_ids();
        let is_all_selected = images.iter().all(|image| ids.contains(&image.image_key));
        if is_all_selected {
            for image in images {
                ids.remove(&image.image_key);
            }
        } else {
            for image in images {
                ids.insert(image.image_key);
            }
        }
        selected_ids.set(ids);
    };

    let handle_page_size_change = move |event: Event<FormData>| {
        let raw = event.value();
        if let Ok(size) = raw.parse::<i32>() {
            let normalized_size = size.clamp(1, 100);
            if normalized_size != page_size() {
                page_size.set(normalized_size);
                current_page.set(1);
                selected_ids.set(HashSet::new());
            }
        }
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

        let delete_list: Vec<String> = ids.iter().cloned().collect();
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

    let current_images = image_store.images();
    let selected_now = selected_ids();
    let selected_count = selected_now.len();
    let all_selected_on_page = !current_images.is_empty()
        && current_images
            .iter()
            .all(|image| selected_now.contains(&image.image_key));

    rsx! {
        div { class: "image-list-page",
            section { class: "image-hero",
                div { class: "image-hero-main",
                    h1 { "上传历史" }
                    p { class: "image-hero-subtitle",
                        "这里展示之前上传的图片，按上传时间倒序排列"
                    }
                }

                div { class: "image-hero-actions",
                    if selected_count > 0 {
                        button {
                            class: "btn btn-danger",
                            disabled: is_loading() || is_deleting(),
                            onclick: handle_delete_selected,
                            "删除所选 ({selected_count})"
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

                div { class: "image-hero-stats",
                    span { class: "stat-pill", "总计 {image_store.total_items()} 张" }
                    span { class: "stat-pill", "当前第 {current_page()} 页" }
                    span { class: "stat-pill", "每页 {page_size()} 张" }
                    if selected_count > 0 {
                        span { class: "stat-pill stat-pill-active", "已选 {selected_count} 张" }
                    }
                }
            }

            div { class: "list-controls",
                div { class: "list-controls-main",
                    label { class: "select-all-toggle",
                        input {
                            r#type: "checkbox",
                            checked: all_selected_on_page,
                            disabled: is_loading() || is_deleting() || current_images.is_empty(),
                            onchange: handle_toggle_all,
                        }
                        span { "全选当前页" }
                    }

                    label { class: "page-size-control",
                        span { "每页" }
                        select {
                            class: "page-size-select",
                            value: "{page_size()}",
                            disabled: is_loading() || is_deleting(),
                            onchange: handle_page_size_change,
                            option { value: "12", "12" }
                            option { value: "20", "20" }
                            option { value: "40", "40" }
                            option { value: "60", "60" }
                            option { value: "100", "100" }
                        }
                        span { "张" }
                    }
                }
                span { class: "page-summary",
                    "按上传时间倒序"
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
                div { class: "error-banner",
                    "{error_message()}"
                }
            }

            div { class: "image-list-wrapper",
                if is_loading() {
                    Loading {}
                } else if current_images.is_empty() {
                    div { class: "empty-state",
                        p { "暂无图片" }
                    }
                } else {
                    ImageGrid {
                        images: current_images.clone(),
                        selected_ids: selected_now.clone(),
                        on_toggle_select: handle_toggle_select,
                        on_download: handle_download,
                        on_delete: handle_delete,
                    }
                }
            }
        }
    }
}
