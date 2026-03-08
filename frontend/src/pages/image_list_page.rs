use dioxus::prelude::*;
use crate::components::image_grid::ImageGrid;
use crate::components::loading::Loading;

/// 图片列表页面组件
#[component]
pub fn ImageListPage() -> Element {
    let mut is_loading = use_signal(|| false);
    let images = use_signal(Vec::new);

    let handle_load_more = move |_: dioxus::events::MouseEvent| async move {
        is_loading.set(true);

        // TODO: 调用实际的图片加载 API
        // 模拟加载过程
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        is_loading.set(false);
    };

    rsx! {
        div { class: "image-list-page",
            header { class: "page-header",
                h1 { "我的图片" }
                button { class: "btn btn-primary",
                    onclick: move |_| {
                        // TODO: 打开上传对话框
                    },
                    "📤 上传图片"
                }
            }

            if is_loading() && images().is_empty() {
                Loading { message: Some("加载中...".to_string()) }
            } else {
                ImageGrid { images: images() }
            }

            // 无限滚动触发点
            div { class: "scroll-trigger", onclick: handle_load_more }
        }
    }
}
