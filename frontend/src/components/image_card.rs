use dioxus::prelude::*;
use crate::types::models::ImageItem;

/// 图片卡片组件
#[component]
pub fn ImageCard(
    image: ImageItem,
    #[props(default)] selected: bool,
    #[props(default)] on_select: EventHandler<bool>,
    #[props(default)] on_download: EventHandler<()>,
    #[props(default)] on_delete: EventHandler<()>,
) -> Element {
    let handle_click = move |_| {
        on_select(!selected);
    };

    let handle_download = move |_| {
        on_download(());
    };

    let handle_delete = move |_| {
        on_delete(());
    };

    rsx! {
        div { class: format!("image-card {}", if selected { "selected" } else { "" }),
            div { class: "image-thumbnail",
                img {
                    src: "{image.thumbnail_url.as_deref().unwrap_or(&image.url)}",
                    alt: "{image.filename}",
                    loading: "lazy"
                }
            }
            div { class: "image-info",
                div { class: "image-name", "{image.original_filename.as_deref().unwrap_or(&image.filename)}" }
                div { class: "image-meta",
                    span { class: "image-size", "{image.size} bytes" }
                    span { class: "image-date", "{image.created_at}" }
                }
            }
            div { class: "image-actions",
                button {
                    class: "btn btn-icon",
                    onclick: handle_click,
                    "👁️"
                }
                button {
                    class: "btn btn-icon",
                    onclick: handle_download,
                    "⬇️"
                }
                button {
                    class: "btn btn-icon btn-danger",
                    onclick: handle_delete,
                    "🗑️"
                }
            }
        }
    }
}
