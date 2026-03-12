use crate::app_context::{use_auth_store, use_image_service, use_image_store, use_toast_store};
use crate::auth_session::{auth_session_expired_message, handle_auth_session_error};
use crate::components::Loading;
use crate::store::ImageCollectionKind;
use crate::types::api::PaginationParams;
use crate::types::models::ImageItem;
use dioxus::prelude::*;

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
    let auth_store = use_auth_store();
    let image_service = use_image_service();
    let image_store = use_image_store();
    let toast_store = use_toast_store();
    let kind = ImageCollectionKind::Deleted;

    let _load_deleted_images = use_resource({
        let auth_store = auth_store.clone();
        let image_service = image_service.clone();
        let image_store = image_store.clone();
        let toast_store = toast_store.clone();
        move || {
            let state = image_store.collection(kind);
            let page = state.current_page.max(1) as i32;
            let size = state.page_size.clamp(1, 100) as i32;
            let _ = state.reload_token;
            let auth_store = auth_store.clone();
            let image_service = image_service.clone();
            let image_store = image_store.clone();
            let toast_store = toast_store.clone();
            async move {
                image_store.set_loading(kind, true);
                image_store.clear_error(kind);

                let params = PaginationParams {
                    page: Some(page),
                    page_size: Some(size),
                    tag: None,
                };

                match image_service.get_deleted_images(params).await {
                    Ok(result) => {
                        if result.data.is_empty() && page > 1 && result.total > 0 {
                            image_store.set_page(kind, (page - 1) as u32);
                            image_store.set_loading(kind, false);
                            return;
                        }
                    }
                    Err(error) => {
                        if handle_auth_session_error(&auth_store, &toast_store, &error) {
                            image_store.set_error_message(kind, auth_session_expired_message());
                        } else {
                            let message = format!("加载回收站失败: {}", error);
                            image_store.set_error_message(kind, message.clone());
                            toast_store.show_error(message);
                        }
                    }
                }

                image_store.set_loading(kind, false);
            }
        }
    });

    let image_store_for_refresh = image_store.clone();
    let handle_refresh = move |_| {
        image_store_for_refresh.mark_for_reload(kind);
    };

    let image_store_for_prev_page = image_store.clone();
    let handle_prev_page = move |_| {
        let state = image_store_for_prev_page.collection(kind);
        if state.is_loading || state.is_processing || state.current_page <= 1 {
            return;
        }
        image_store_for_prev_page.set_page(kind, state.current_page - 1);
    };

    let image_store_for_next_page = image_store.clone();
    let handle_next_page = move |_| {
        let state = image_store_for_next_page.collection(kind);
        if state.is_loading || state.is_processing || !state.has_more {
            return;
        }
        image_store_for_next_page.set_page(kind, state.current_page + 1);
    };

    let image_store_for_toggle_all = image_store.clone();
    let handle_toggle_all = move |_| {
        image_store_for_toggle_all.toggle_all_visible(kind);
    };

    let image_store_for_page_size = image_store.clone();
    let handle_page_size_change = move |event: Event<FormData>| {
        let raw = event.value();
        if let Ok(size) = raw.parse::<u32>() {
            let normalized_size = size.clamp(1, 100);
            let state = image_store_for_page_size.collection(kind);
            if normalized_size != state.page_size {
                image_store_for_page_size.set_page_size(kind, normalized_size);
                image_store_for_page_size.set_page(kind, 1);
                image_store_for_page_size.clear_selection(kind);
            }
        }
    };

    let image_service_for_batch_restore = image_service.clone();
    let auth_store_for_batch_restore = auth_store.clone();
    let image_store_for_batch_restore = image_store.clone();
    let toast_store_for_batch_restore = toast_store.clone();
    let handle_restore_selected = move |_| {
        let state = image_store_for_batch_restore.collection(kind);
        if state.is_processing || state.selected_ids.is_empty() {
            return;
        }

        let restore_list: Vec<String> = state.selected_ids.iter().cloned().collect();
        let count = restore_list.len();
        let image_service = image_service_for_batch_restore.clone();
        let auth_store = auth_store_for_batch_restore.clone();
        let image_store = image_store_for_batch_restore.clone();
        let toast_store = toast_store_for_batch_restore.clone();
        spawn(async move {
            image_store.set_processing(kind, true);

            match image_service.restore_images(restore_list).await {
                Ok(_) => {
                    toast_store.show_success(format!("已恢复 {} 张图片", count));
                    image_store.clear_selection(kind);
                    image_store.mark_for_reload(kind);
                }
                Err(error) => {
                    if handle_auth_session_error(&auth_store, &toast_store, &error) {
                        image_store.set_error_message(kind, auth_session_expired_message());
                    } else {
                        toast_store.show_error(format!("批量恢复失败: {}", error));
                    }
                }
            }

            image_store.set_processing(kind, false);
        });
    };

    let image_service_for_batch_delete = image_service.clone();
    let auth_store_for_batch_delete = auth_store.clone();
    let image_store_for_batch_delete = image_store.clone();
    let toast_store_for_batch_delete = toast_store.clone();
    let handle_delete_selected = move |_| {
        let state = image_store_for_batch_delete.collection(kind);
        if state.is_processing || state.selected_ids.is_empty() {
            return;
        }

        let delete_list: Vec<String> = state.selected_ids.iter().cloned().collect();
        let count = delete_list.len();
        if !confirm_permanent_delete(&format!(
            "确定要彻底删除选中的 {} 张图片吗？此操作不可撤销。",
            count
        )) {
            return;
        }

        let image_service = image_service_for_batch_delete.clone();
        let auth_store = auth_store_for_batch_delete.clone();
        let image_store = image_store_for_batch_delete.clone();
        let toast_store = toast_store_for_batch_delete.clone();
        spawn(async move {
            image_store.set_processing(kind, true);

            match image_service.delete_images(delete_list, true).await {
                Ok(_) => {
                    toast_store.show_success(format!("已彻底删除 {} 张图片", count));
                    image_store.clear_selection(kind);
                    image_store.mark_for_reload(kind);
                }
                Err(error) => {
                    if handle_auth_session_error(&auth_store, &toast_store, &error) {
                        image_store.set_error_message(kind, auth_session_expired_message());
                    } else {
                        toast_store.show_error(format!("批量彻底删除失败: {}", error));
                    }
                }
            }

            image_store.set_processing(kind, false);
        });
    };

    let state = image_store.collection(kind);
    let selected_count = state.selected_ids.len();
    let all_selected_on_page = !state.images.is_empty()
        && state
            .images
            .iter()
            .all(|image| state.selected_ids.contains(&image.image_key));

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
                            disabled: state.is_loading || state.is_processing,
                            onclick: handle_restore_selected,
                            "恢复所选 ({selected_count})"
                        }
                        button {
                            class: "btn btn-danger",
                            disabled: state.is_loading || state.is_processing,
                            onclick: handle_delete_selected,
                            "彻底删除 ({selected_count})"
                        }
                    }
                    button {
                        class: "btn btn-primary",
                        disabled: state.is_loading || state.is_processing,
                        onclick: handle_refresh,
                        if state.is_loading { "刷新中..." } else { "刷新" }
                    }
                }

                div { class: "image-hero-stats",
                    span { class: "stat-pill", "回收站共 {state.total_items} 张" }
                    span { class: "stat-pill", "当前第 {state.current_page} 页" }
                    span { class: "stat-pill", "每页 {state.page_size} 张" }
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
                            disabled: state.is_loading || state.is_processing || state.images.is_empty(),
                            onchange: handle_toggle_all,
                        }
                        span { "全选当前页" }
                    }

                    label { class: "page-size-control",
                        span { "每页" }
                        select {
                            class: "page-size-select",
                            value: "{state.page_size}",
                            disabled: state.is_loading || state.is_processing,
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
                        disabled: state.is_loading || state.is_processing || state.current_page <= 1,
                        onclick: handle_prev_page,
                        "上一页"
                    }
                    button {
                        class: "btn",
                        disabled: state.is_loading || state.is_processing || !state.has_more,
                        onclick: handle_next_page,
                        "下一页"
                    }
                }
            }

            if !state.error_message.is_empty() {
                div { class: "error-banner", "{state.error_message}" }
            }

            div { class: "image-list-wrapper",
                if state.is_loading {
                    Loading {}
                } else if state.images.is_empty() {
                    div { class: "empty-state",
                        h3 { "回收站为空" }
                    }
                } else {
                    div { class: "image-grid trash-grid",
                        {state.images.iter().map(|image| {
                            let image_key = image.image_key.clone();
                            let image_for_restore = image.clone();
                            let image_for_delete = image.clone();
                            let image_store_for_select = image_store.clone();
                            let image_service_for_restore = image_service.clone();
                            let auth_store_for_restore = auth_store.clone();
                            let image_store_for_restore = image_store.clone();
                            let toast_store_for_restore = toast_store.clone();
                            let image_service_for_delete = image_service.clone();
                            let auth_store_for_delete = auth_store.clone();
                            let image_store_for_delete = image_store.clone();
                            let toast_store_for_delete = toast_store.clone();

                            rsx! {
                                DeletedImageCard {
                                    key: "{image.image_key}",
                                    image: image.clone(),
                                    selected: state.selected_ids.contains(&image.image_key),
                                    on_select: move |_| {
                                        image_store_for_select.toggle_selection(kind, &image_key)
                                    },
                                    on_restore: move |_| {
                                        let state = image_store_for_restore.collection(kind);
                                        if state.is_processing {
                                            return;
                                        }

                                        let image_service = image_service_for_restore.clone();
                                        let auth_store = auth_store_for_restore.clone();
                                        let image_store = image_store_for_restore.clone();
                                        let toast_store = toast_store_for_restore.clone();
                                        let image = image_for_restore.clone();
                                        spawn(async move {
                                            image_store.set_processing(kind, true);

                                            match image_service
                                                .restore_images(vec![image.image_key.clone()])
                                                .await
                                            {
                                                Ok(_) => {
                                                    toast_store.show_success(format!(
                                                        "已恢复: {}",
                                                        image.display_name()
                                                    ));
                                                    image_store.remove_selection(kind, &image.image_key);
                                                    image_store.mark_for_reload(kind);
                                                }
                                                Err(error) => {
                                                    if handle_auth_session_error(&auth_store, &toast_store, &error) {
                                                        image_store.set_error_message(kind, auth_session_expired_message());
                                                    } else {
                                                        toast_store.show_error(format!("恢复失败: {}", error));
                                                    }
                                                }
                                            }

                                            image_store.set_processing(kind, false);
                                        });
                                    },
                                    on_delete: move |_| {
                                        let state = image_store_for_delete.collection(kind);
                                        if state.is_processing {
                                            return;
                                        }
                                        if !confirm_permanent_delete(&format!(
                                            "确定要彻底删除“{}”吗？此操作不可撤销。",
                                            image_for_delete.display_name()
                                        )) {
                                            return;
                                        }

                                        let image_service = image_service_for_delete.clone();
                                        let auth_store = auth_store_for_delete.clone();
                                        let image_store = image_store_for_delete.clone();
                                        let toast_store = toast_store_for_delete.clone();
                                        let image = image_for_delete.clone();
                                        spawn(async move {
                                            image_store.set_processing(kind, true);

                                            match image_service
                                                .delete_images(vec![image.image_key.clone()], true)
                                                .await
                                            {
                                                Ok(_) => {
                                                    toast_store.show_success(format!(
                                                        "已彻底删除: {}",
                                                        image.display_name()
                                                    ));
                                                    image_store.remove_selection(kind, &image.image_key);
                                                    image_store.mark_for_reload(kind);
                                                }
                                                Err(error) => {
                                                    if handle_auth_session_error(&auth_store, &toast_store, &error) {
                                                        image_store.set_error_message(kind, auth_session_expired_message());
                                                    } else {
                                                        toast_store.show_error(format!(
                                                            "彻底删除失败: {}",
                                                            error
                                                        ));
                                                    }
                                                }
                                            }

                                            image_store.set_processing(kind, false);
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
