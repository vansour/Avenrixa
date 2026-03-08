use crate::types::models::ImageItem;
use dioxus::prelude::*;

/// 图片卡片组件
#[component]
pub fn ImageCard(
    image: ImageItem,
    #[props(default)] selected: bool,
    #[props(default)] on_select: EventHandler<()>,
    #[props(default)] on_download: EventHandler<()>,
    #[props(default)] on_delete: EventHandler<()>,
) -> Element {
    let handle_click = move |_| {
        on_select(());
    };

    let handle_download = move |_| {
        on_download(());
    };

    let handle_delete = move |_| {
        on_delete(());
    };

    // 预计算值以避免在 rsx! 中调用函数
    let thumbnail_url = match image.thumbnail_url() {
        Some(url) => url,
        None => image.url(),
    };
    let original_filename = image.original_filename.as_ref().unwrap_or(&image.filename);
    let size_formatted = image.size_formatted();

    rsx! {
        div { class: format!("image-card {}", if selected { "selected" } else { "" }),
            div { class: "image-thumbnail",
                img {
                    src: "{thumbnail_url}",
                    alt: "{image.filename}",
                    loading: "lazy"
                }
            }
            div { class: "image-info",
                div { class: "image-name", "{original_filename}" }
                div { class: "image-meta",
                    span { class: "image-size", "{size_formatted}" }
                    span { class: "image-date", "{image.created_at}" }
                }
            }
            div { class: "image-actions",
                button {
                    class: "btn btn-icon",
                    onclick: handle_click,
                    "选择"
                }
                button {
                    class: "btn btn-icon",
                    onclick: handle_download,
                    "下载"
                }
                button {
                    class: "btn btn-icon btn-danger",
                    onclick: handle_delete,
                    "删除"
                }
            }
        }
    }
}
