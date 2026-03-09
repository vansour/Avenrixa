use crate::components::image_card::ImageCard;
use crate::types::models::ImageItem;
use dioxus::prelude::*;
use std::collections::HashSet;

/// 图片网格组件
#[component]
pub fn ImageGrid(
    images: Vec<ImageItem>,
    #[props(default)] selected_ids: HashSet<String>,
    #[props(default)] on_toggle_select: EventHandler<String>,
    #[props(default)] on_download: EventHandler<ImageItem>,
    #[props(default)] on_delete: EventHandler<ImageItem>,
) -> Element {
    rsx! {
        div { class: "image-grid",
            if images.is_empty() {
                div { class: "empty-state",
                    h3 { "暂无图片" }
                    p { "上传图片开始使用吧！" }
                }
            } else {
                {images.iter().map(|image| {
                    let image_for_select = image.clone();
                    let image_for_download = image.clone();
                    let image_for_delete = image.clone();

                    rsx! {
                        ImageCard {
                            key: "{image.image_key}",
                            image: image.clone(),
                            selected: selected_ids.contains(&image.image_key),
                            on_select: move |_| on_toggle_select.call(image_for_select.image_key.clone()),
                            on_download: move |_| on_download.call(image_for_download.clone()),
                            on_delete: move |_| on_delete.call(image_for_delete.clone()),
                        }
                    }
                })}
            }
        }
    }
}
