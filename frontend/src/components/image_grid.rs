use crate::components::image_card::ImageCard;
use crate::types::models::ImageItem;
use dioxus::prelude::*;

/// 图片网格组件
#[component]
pub fn ImageGrid(images: Vec<ImageItem>) -> Element {
    rsx! {
        div { class: "image-grid",
            if images.is_empty() {
                div { class: "empty-state",
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
