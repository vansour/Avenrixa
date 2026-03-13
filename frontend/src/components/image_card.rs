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
    let handle_select = move |_| {
        on_select(());
    };

    let handle_download = move |_| {
        on_download(());
    };

    let handle_delete = move |_| {
        on_delete(());
    };

    // 预计算值以避免在 rsx! 中调用函数
    let thumbnail_url = image.thumbnail_url();
    let display_name = image.display_name();
    let size_formatted = image.size_formatted();
    let created_at_label = image.created_at_label();

    rsx! {
        article { class: format!("image-card {}", if selected { "selected" } else { "" }),
            div { class: "image-thumbnail",
                img {
                    src: "{thumbnail_url}",
                    alt: "{image.filename}",
                    loading: "lazy"
                }
                label { class: "image-select",
                    input {
                        r#type: "checkbox",
                        checked: selected,
                        onclick: handle_select,
                        "aria-label": "选择图片 {display_name}"
                    }
                    span { class: "image-select-indicator" }
                }
                div { class: "image-chip",
                    "{image.format.to_uppercase()}"
                }
            }
            div { class: "image-content",
                div { class: "image-info",
                    div { class: "image-name", "{display_name}" }
                    div { class: "image-meta",
                        span { class: "image-size", "{size_formatted}" }
                        span { class: "image-date", "{created_at_label}" }
                    }
                }
                div { class: "image-actions",
                    button {
                        class: "btn btn-card",
                        onclick: handle_download,
                        "下载"
                    }
                    button {
                        class: "btn btn-card btn-card-danger",
                        onclick: handle_delete,
                        "永久删除"
                    }
                }
            }
        }
    }
}
