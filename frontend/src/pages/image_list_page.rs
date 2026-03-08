use crate::app_context::AppContext;
use crate::components::image_grid::ImageGrid;
use crate::types::api::PaginationParams;
use dioxus::prelude::*;

/// 图片列表页面组件
#[component]
pub fn ImageListPage() -> Element {
    let app_context = AppContext::new("http://localhost:3000".to_string());
    let image_store = app_context.image_store.clone();
    let image_service = app_context.get_image_service();

    // Clone用于闭包
    let image_store_for_future = image_store.clone();
    let image_service_for_future = image_service.clone();

    // 页面加载时获取图片数据
    let _load_images = use_future(move || {
        let image_service = image_service_for_future.clone();
        let image_store = image_store_for_future.clone();
        async move {
            let params = PaginationParams {
                page: Some(1),
                page_size: Some(20),
                sort_by: "created_at".to_string(),
                sort_order: "DESC".to_string(),
                search: None,
                category_id: None,
                tag: None,
                cursor: None,
            };

            if let Ok(result) = image_service.get_images(params).await {
                image_store.set_images(result.data);
            }
        }
    });

    rsx! {
        div { class: "image-list-page",
            header { class: "page-header",
                h1 { "我的图片" }
                button { class: "btn btn-primary",
                    "上传图片"
                }
            }

            div { class: "image-list-wrapper",
                if image_store.images_is_empty() {
                    div { class: "empty-state",
                        p { "暂无图片" }
                        button { class: "btn btn-primary",
                            "上传图片"
                        }
                    }
                } else {
                    ImageGrid { images: image_store.images() }
                }
            }
        }
    }
}
