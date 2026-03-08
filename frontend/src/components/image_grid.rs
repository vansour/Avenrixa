use dioxus::prelude::*;
use crate::components::image_card::ImageCard;
use crate::types::models::ImageItem;

/// 图片网格组件
#[component]
pub fn ImageGrid(
    images: Vec<ImageItem>,
) -> Element {
    rsx! {
        div { class: "image-grid",
            if images.is_empty() {
                div { class: "empty-state",
                    div { class: "empty-icon", "🖼️" }
                    h3 { "暂无图片" }
                    p { "上传图片开始使用吧！" }
                }
            } else {
                for image in images.iter() {
                    ImageCard {
                        image: image.clone(),
                        selected: false,
                        on_select: |_| {},
                        on_download: |_| {},
                        on_delete: |_| {},
                    }
                }
            }
        }
    }
}
