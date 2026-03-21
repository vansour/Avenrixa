use crate::app_context::{use_auth_store, use_image_service, use_image_store, use_toast_store};
use crate::auth_session::{auth_session_expired_message, handle_auth_session_error};
use crate::components::{ImageGrid, Loading};
use crate::store::ImageCollectionKind;
use crate::types::api::CursorPaginationParams;
use crate::types::models::ImageItem;
use dioxus::prelude::*;

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

#[cfg(target_arch = "wasm32")]
fn confirm_delete(message: &str) -> bool {
    web_sys::window()
        .and_then(|window| window.confirm_with_message(message).ok())
        .unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
fn confirm_delete(_message: &str) -> bool {
    true
}

/// 图片列表页面组件
#[component]
pub fn ImageListPage() -> Element {
    let auth_store = use_auth_store();
    let image_service = use_image_service();
    let image_store = use_image_store();
    let toast_store = use_toast_store();
    let kind = ImageCollectionKind::Active;
    let state = image_store.collection(kind);
    let request_key = (
        state.current_cursor.clone(),
        state.page_size.clamp(1, 100),
        state.reload_token,
    );
    let mut last_request_key = use_signal(|| None::<(Option<String>, u32, u64)>);

    use_effect({
        let auth_store = auth_store.clone();
        let image_service = image_service.clone();
        let image_store = image_store.clone();
        let toast_store = toast_store.clone();
        move || {
            if last_request_key() == Some(request_key.clone()) {
                return;
            }

            last_request_key.set(Some(request_key.clone()));

            let auth_store = auth_store.clone();
            let image_service = image_service.clone();
            let image_store = image_store.clone();
            let toast_store = toast_store.clone();
            let cursor = request_key.0.clone();
            let size = request_key.1 as i32;
            let page = image_store.collection(kind).current_page;
            spawn(async move {
                image_store.set_loading(kind, true);
                image_store.clear_error(kind);

                let params = CursorPaginationParams {
                    cursor,
                    limit: Some(size),
                };

                match image_service.get_images(params).await {
                    Ok(result) => {
                        if result.data.is_empty() && page > 1 {
                            image_store.go_to_previous_page(kind);
                            image_store.set_loading(kind, false);
                            return;
                        }
                    }
                    Err(error) => {
                        if handle_auth_session_error(&auth_store, &toast_store, &error) {
                            image_store.set_error_message(kind, auth_session_expired_message());
                        } else {
                            let message = format!("加载图片失败: {}", error);
                            image_store.set_error_message(kind, message.clone());
                            toast_store.show_error(message);
                        }
                    }
                }

                image_store.set_loading(kind, false);
            });
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
        image_store_for_prev_page.go_to_previous_page(kind);
    };

    let image_store_for_next_page = image_store.clone();
    let handle_next_page = move |_| {
        let state = image_store_for_next_page.collection(kind);
        if state.is_loading || state.is_processing || !state.has_more {
            return;
        }
        image_store_for_next_page.go_to_next_page(kind);
    };

    let image_store_for_toggle_select = image_store.clone();
    let handle_toggle_select = move |image_key: String| {
        image_store_for_toggle_select.toggle_selection(kind, &image_key);
    };

    let toast_store_for_download = toast_store.clone();
    let handle_download = move |image: ImageItem| {
        let url = image.url();
        if !open_in_new_tab(&url) {
            toast_store_for_download.show_info(format!("图片地址: {}", url));
        }
    };

    let image_service_for_delete = image_service.clone();
    let auth_store_for_delete = auth_store.clone();
    let image_store_for_delete = image_store.clone();
    let toast_store_for_delete = toast_store.clone();
    let handle_delete = move |image: ImageItem| {
        let state = image_store_for_delete.collection(kind);
        if state.is_processing {
            return;
        }
        if !confirm_delete(&format!(
            "确定要永久删除 {} 吗？删除后无法恢复。",
            image.display_name()
        )) {
            return;
        }

        let image_service = image_service_for_delete.clone();
        let auth_store = auth_store_for_delete.clone();
        let image_store = image_store_for_delete.clone();
        let toast_store = toast_store_for_delete.clone();
        spawn(async move {
            image_store.set_processing(kind, true);

            match image_service
                .delete_images(vec![image.image_key.clone()])
                .await
            {
                Ok(_) => {
                    toast_store.show_success(format!("已永久删除: {}", image.display_name()));
                    image_store.remove_selection(kind, &image.image_key);
                    image_store.mark_for_reload(kind);
                }
                Err(error) => {
                    if handle_auth_session_error(&auth_store, &toast_store, &error) {
                        image_store.set_error_message(kind, auth_session_expired_message());
                    } else {
                        toast_store.show_error(format!("删除失败: {}", error));
                    }
                }
            }

            image_store.set_processing(kind, false);
        });
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
                image_store_for_page_size.reset_pagination(kind);
                image_store_for_page_size.clear_selection(kind);
            }
        }
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
        if !confirm_delete(&format!(
            "确定要永久删除选中的 {} 张图片吗？删除后无法恢复。",
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

            match image_service.delete_images(delete_list).await {
                Ok(_) => {
                    toast_store.show_success(format!("已永久删除 {} 张图片", count));
                    image_store.clear_selection(kind);
                    image_store.mark_for_reload(kind);
                }
                Err(error) => {
                    if handle_auth_session_error(&auth_store, &toast_store, &error) {
                        image_store.set_error_message(kind, auth_session_expired_message());
                    } else {
                        toast_store.show_error(format!("批量删除失败: {}", error));
                    }
                }
            }

            image_store.set_processing(kind, false);
        });
    };

    let selected_count = state.selected_ids.len();
    let all_selected_on_page = !state.images.is_empty()
        && state
            .images
            .iter()
            .all(|image| state.selected_ids.contains(&image.image_key));
    let page_summary = "默认按上传时间倒序";

    rsx! {
        div { class: "image-list-page",
            section { class: "image-hero",
                div { class: "image-hero-main",
                    h1 { "上传历史" }
                    p { class: "image-hero-subtitle",
                        "按上传时间查看已上传的图片，删除后不可恢复"
                    }
                }

                div { class: "image-hero-actions",
                    if selected_count > 0 {
                        button {
                            class: "btn btn-danger",
                            disabled: state.is_loading || state.is_processing,
                            onclick: handle_delete_selected,
                            "永久删除所选 ({selected_count})"
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
                    span { class: "stat-pill", "当前第 {state.current_page} 页" }
                    span { class: "stat-pill", "本页 {state.images.len()} 张" }
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
                span { class: "page-summary", "{page_summary}" }
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
                        h3 { "暂无图片" }
                        p { "上传图片开始使用吧！" }
                    }
                } else {
                    ImageGrid {
                        images: state.images.clone(),
                        selected_ids: state.selected_ids.clone(),
                        on_toggle_select: handle_toggle_select,
                        on_download: handle_download,
                        on_delete: handle_delete,
                    }
                }
            }
        }
    }
}
