use crate::app_context::{use_image_service, use_toast_store};
use crate::components::Loading;
use crate::types::api::PaginationParams;
use crate::types::models::ImageItem;
use dioxus::prelude::*;
use std::collections::HashSet;

#[cfg(target_arch = "wasm32")]
fn confirm_permanent_delete(message: &str) -> bool {
    web_sys::window()
        .and_then(|window| window.confirm_with_message(message).ok())
        .unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
fn confirm_permanent_delete(_message: &str) -> bool {
    true
}

/// 回收站页面组件
#[component]
pub fn DeletedImagesPage() -> Element {
    let image_service = use_image_service();
    let toast_store = use_toast_store();

    let mut deleted_images = use_signal(Vec::<ImageItem>::new);
    let mut total_items = use_signal(|| 0_i64);
    let mut has_next = use_signal(|| false);
    let mut is_loading = use_signal(|| false);
    let mut is_processing = use_signal(|| false);
    let mut error_message = use_signal(String::new);
    let mut selected_ids = use_signal(HashSet::<String>::new);
    let mut reload_tick = use_signal(|| 0_u64);
    let mut current_page = use_signal(|| 1_i32);
    let mut page_size = use_signal(|| 20_i32);

    let _load_deleted_images = use_resource({
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
                    category_id: None,
                    tag: None,
                };

                match image_service.get_deleted_images(params).await {
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

                        deleted_images.set(result.data);
                        total_items.set(result.total);
                        has_next.set(result.has_next);
                        selected_ids.set(HashSet::new());
                    }
                    Err(error) => {
                        let message = format!("加载回收站失败: {}", error);
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

    let handle_next_page = move |_| {
        if is_loading() || !has_next() {
            return;
        }
        current_page.set(current_page() + 1);
    };

    let handle_toggle_all = move |_| {
        let images = deleted_images();
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

    let image_service_for_batch_restore = image_service.clone();
    let toast_store_for_batch_restore = toast_store.clone();
    let handle_restore_selected = move |_| {
        if is_processing() {
            return;
        }

        let ids = selected_ids();
        if ids.is_empty() {
            return;
        }

        let restore_list: Vec<String> = ids.iter().cloned().collect();
        let count = restore_list.len();
        let image_service = image_service_for_batch_restore.clone();
        let toast_store = toast_store_for_batch_restore.clone();
        spawn(async move {
            is_processing.set(true);

            match image_service.restore_images(restore_list).await {
                Ok(_) => {
                    toast_store.show_success(format!("已恢复 {} 张图片", count));
                    selected_ids.set(HashSet::new());
                    reload_tick.set(reload_tick().wrapping_add(1));
                }
                Err(error) => {
                    toast_store.show_error(format!("批量恢复失败: {}", error));
                }
            }

            is_processing.set(false);
        });
    };

    let image_service_for_batch_delete = image_service.clone();
    let toast_store_for_batch_delete = toast_store.clone();
    let handle_delete_selected = move |_| {
        if is_processing() {
            return;
        }

        let ids = selected_ids();
        if ids.is_empty() {
            return;
        }

        let delete_list: Vec<String> = ids.iter().cloned().collect();
        let count = delete_list.len();
        if !confirm_permanent_delete(&format!(
            "确定要彻底删除选中的 {} 张图片吗？此操作不可撤销。",
            count
        )) {
            return;
        }

        let image_service = image_service_for_batch_delete.clone();
        let toast_store = toast_store_for_batch_delete.clone();
        spawn(async move {
            is_processing.set(true);

            match image_service.delete_images(delete_list, true).await {
                Ok(_) => {
                    toast_store.show_success(format!("已彻底删除 {} 张图片", count));
                    selected_ids.set(HashSet::new());
                    reload_tick.set(reload_tick().wrapping_add(1));
                }
                Err(error) => {
                    toast_store.show_error(format!("批量彻底删除失败: {}", error));
                }
            }

            is_processing.set(false);
        });
    };

    let current_images = deleted_images();
    let selected_now = selected_ids();
    let selected_count = selected_now.len();
    let all_selected_on_page = !current_images.is_empty()
        && current_images
            .iter()
            .all(|image| selected_now.contains(&image.image_key));

    rsx! {
        div { class: "image-list-page",
            section { class: "image-hero image-hero-trash",
                div { class: "image-hero-main",
                    h1 { "回收站" }
                }

                div { class: "image-hero-actions",
                    if selected_count > 0 {
                        button {
                            class: "btn btn-ghost",
                            disabled: is_loading() || is_processing(),
                            onclick: handle_restore_selected,
                            "恢复所选 ({selected_count})"
                        }
                        button {
                            class: "btn btn-danger",
                            disabled: is_loading() || is_processing(),
                            onclick: handle_delete_selected,
                            "彻底删除 ({selected_count})"
                        }
                    }
                    button {
                        class: "btn btn-primary",
                        disabled: is_loading() || is_processing(),
                        onclick: handle_refresh,
                        if is_loading() { "刷新中..." } else { "刷新" }
                    }
                }

                div { class: "image-hero-stats",
                    span { class: "stat-pill", "回收站共 {total_items()} 张" }
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
                            disabled: is_loading() || is_processing() || current_images.is_empty(),
                            onchange: handle_toggle_all,
                        }
                        span { "全选当前页" }
                    }

                    label { class: "page-size-control",
                        span { "每页" }
                        select {
                            class: "page-size-select",
                            value: "{page_size()}",
                            disabled: is_loading() || is_processing(),
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

                div { class: "page-actions",
                    button {
                        class: "btn",
                        disabled: is_loading() || is_processing() || current_page() <= 1,
                        onclick: handle_prev_page,
                        "上一页"
                    }
                    button {
                        class: "btn",
                        disabled: is_loading() || is_processing() || !has_next(),
                        onclick: handle_next_page,
                        "下一页"
                    }
                }
            }

            if !error_message().is_empty() {
                div { class: "error-banner", "{error_message()}" }
            }

            div { class: "image-list-wrapper",
                if is_loading() {
                    Loading {}
                } else if current_images.is_empty() {
                    div { class: "empty-state",
                        h3 { "回收站为空" }
                    }
                } else {
                    div { class: "image-grid trash-grid",
                        {current_images.iter().map(|image| {
                            let image_for_restore = image.clone();
                            let image_for_delete = image.clone();
                            let image_key = image.image_key.clone();
                            let image_service_for_restore = image_service.clone();
                            let toast_store_for_restore = toast_store.clone();
                            let image_service_for_delete = image_service.clone();
                            let toast_store_for_delete = toast_store.clone();

                            rsx! {
                                DeletedImageCard {
                                    key: "{image.image_key}",
                                    image: image.clone(),
                                    selected: selected_now.contains(&image.image_key),
                                    on_select: move |_| {
                                        let mut ids = selected_ids();
                                        if !ids.insert(image_key.clone()) {
                                            ids.remove(&image_key);
                                        }
                                        selected_ids.set(ids);
                                    },
                                    on_restore: move |_| {
                                        if is_processing() {
                                            return;
                                        }

                                        let image_service = image_service_for_restore.clone();
                                        let toast_store = toast_store_for_restore.clone();
                                        let image = image_for_restore.clone();
                                        spawn(async move {
                                            is_processing.set(true);

                                            match image_service
                                                .restore_images(vec![image.image_key.clone()])
                                                .await
                                            {
                                                Ok(_) => {
                                                    toast_store.show_success(format!(
                                                        "已恢复: {}",
                                                        image.display_name()
                                                    ));
                                                    let mut ids = selected_ids();
                                                    ids.remove(&image.image_key);
                                                    selected_ids.set(ids);
                                                    reload_tick.set(reload_tick().wrapping_add(1));
                                                }
                                                Err(error) => {
                                                    toast_store.show_error(format!(
                                                        "恢复失败: {}",
                                                        error
                                                    ));
                                                }
                                            }

                                            is_processing.set(false);
                                        });
                                    },
                                    on_delete: move |_| {
                                        if is_processing() {
                                            return;
                                        }
                                        if !confirm_permanent_delete(&format!(
                                            "确定要彻底删除“{}”吗？此操作不可撤销。",
                                            image_for_delete.display_name()
                                        )) {
                                            return;
                                        }

                                        let image_service = image_service_for_delete.clone();
                                        let toast_store = toast_store_for_delete.clone();
                                        let image = image_for_delete.clone();
                                        spawn(async move {
                                            is_processing.set(true);

                                            match image_service
                                                .delete_images(vec![image.image_key.clone()], true)
                                                .await
                                            {
                                                Ok(_) => {
                                                    toast_store.show_success(format!(
                                                        "已彻底删除: {}",
                                                        image.display_name()
                                                    ));
                                                    let mut ids = selected_ids();
                                                    ids.remove(&image.image_key);
                                                    selected_ids.set(ids);
                                                    reload_tick.set(reload_tick().wrapping_add(1));
                                                }
                                                Err(error) => {
                                                    toast_store.show_error(format!(
                                                        "彻底删除失败: {}",
                                                        error
                                                    ));
                                                }
                                            }

                                            is_processing.set(false);
                                        });
                                    },
                                }
                            }
                        })}
                    }
                }
            }
        }
    }
}

#[component]
fn DeletedImageCard(
    image: ImageItem,
    #[props(default)] selected: bool,
    #[props(default)] on_select: EventHandler<()>,
    #[props(default)] on_restore: EventHandler<()>,
    #[props(default)] on_delete: EventHandler<()>,
) -> Element {
    let display_name = image.display_name();
    let size_formatted = image.size_formatted();
    let created_at_label = image.created_at_label();
    let deleted_at_label = image
        .deleted_at_label()
        .unwrap_or_else(|| "未知时间".to_string());
    let format_label = image.format.to_uppercase();

    rsx! {
        article { class: format!("image-card {}", if selected { "selected" } else { "" }),
            div { class: "image-thumbnail image-thumbnail-placeholder",
                label { class: "image-select",
                    input {
                        r#type: "checkbox",
                        checked: selected,
                        onclick: move |_| on_select.call(()),
                        "aria-label": "选择图片 {display_name}"
                    }
                    span { class: "image-select-indicator" }
                }
                div { class: "image-chip image-chip-trash", "DELETED" }
                div { class: "trash-thumbnail-glyph", "{format_label}" }
                div { class: "trash-thumbnail-note", "待恢复资产" }
            }

            div { class: "image-info",
                div { class: "image-name", "{display_name}" }
                div { class: "image-meta",
                    span { class: "image-size", "{size_formatted}" }
                    span { class: "image-date", "上传于 {created_at_label}" }
                }
                div { class: "trash-note", "删除于 {deleted_at_label}" }
            }

            div { class: "image-actions",
                button {
                    class: "btn btn-card btn-card-primary",
                    onclick: move |_| on_restore.call(()),
                    "恢复"
                }
                button {
                    class: "btn btn-card btn-card-danger",
                    onclick: move |_| on_delete.call(()),
                    "彻底删除"
                }
            }
        }
    }
}
