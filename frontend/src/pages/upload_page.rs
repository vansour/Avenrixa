use crate::app_context::{use_image_service, use_toast_store};
use dioxus::html::FileData;
use dioxus::prelude::*;

fn format_bytes(size: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * KB;
    const GB: f64 = 1024.0 * MB;

    let size_f = size as f64;
    if size_f >= GB {
        format!("{:.2} GB", size_f / GB)
    } else if size_f >= MB {
        format!("{:.2} MB", size_f / MB)
    } else if size_f >= KB {
        format!("{:.2} KB", size_f / KB)
    } else {
        format!("{} B", size)
    }
}

#[cfg(target_arch = "wasm32")]
fn extension_from_mime(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "image/bmp" => "bmp",
        "image/avif" => "avif",
        "image/heif" => "heif",
        "image/heic" => "heic",
        _ => "png",
    }
}

#[cfg(target_arch = "wasm32")]
async fn read_image_from_clipboard() -> Result<(String, Option<String>, Vec<u8>), String> {
    use js_sys::{Array, Date, Function, Reflect, Uint8Array};
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or_else(|| "无法访问浏览器窗口".to_string())?;
    let navigator = window.navigator();
    let clipboard_js = Reflect::get(navigator.as_ref(), &JsValue::from_str("clipboard"))
        .map_err(|_| "无法访问浏览器剪贴板接口".to_string())?;

    if clipboard_js.is_null() || clipboard_js.is_undefined() {
        return Err("当前浏览器不支持剪贴板图片读取，请改用文件上传".to_string());
    }

    let read_method = Reflect::get(&clipboard_js, &JsValue::from_str("read"))
        .map_err(|_| "当前浏览器不支持剪贴板图片读取，请改用文件上传".to_string())?;
    if read_method.dyn_ref::<Function>().is_none() {
        return Err("当前浏览器不支持剪贴板图片读取，请改用文件上传".to_string());
    }

    let clipboard: web_sys::Clipboard = clipboard_js
        .dyn_into()
        .map_err(|_| "当前浏览器不支持剪贴板图片读取，请改用文件上传".to_string())?;
    let items_js = JsFuture::from(clipboard.read())
        .await
        .map_err(|_| "读取剪贴板失败，请允许浏览器访问剪贴板".to_string())?;

    let items = Array::from(&items_js);
    for item_js in items.iter() {
        let item: web_sys::ClipboardItem = item_js
            .dyn_into()
            .map_err(|_| "解析剪贴板条目失败".to_string())?;

        let types = item.types();
        for mime_js in types.iter() {
            let Some(mime) = mime_js.as_string() else {
                continue;
            };
            if !mime.starts_with("image/") {
                continue;
            }

            let blob_js = JsFuture::from(item.get_type(&mime))
                .await
                .map_err(|_| "读取剪贴板图片失败".to_string())?;
            let blob: web_sys::Blob = blob_js
                .dyn_into()
                .map_err(|_| "转换剪贴板图片格式失败".to_string())?;

            let buffer_js = JsFuture::from(blob.array_buffer())
                .await
                .map_err(|_| "读取剪贴板图片内容失败".to_string())?;
            let buffer = Uint8Array::new(&buffer_js);
            let mut bytes = vec![0_u8; buffer.length() as usize];
            buffer.copy_to(&mut bytes);

            if bytes.is_empty() {
                continue;
            }

            let filename = format!(
                "paste-{}.{}",
                Date::now() as u64,
                extension_from_mime(&mime)
            );
            return Ok((filename, Some(mime), bytes));
        }
    }

    Err("剪贴板中没有可上传的图片".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
async fn read_image_from_clipboard() -> Result<(String, Option<String>, Vec<u8>), String> {
    Err("当前运行环境不支持剪贴板图片读取".to_string())
}

/// 上传页面组件
#[component]
pub fn UploadPage() -> Element {
    let image_service = use_image_service();
    let toast_store = use_toast_store();

    let mut selected_file = use_signal(|| None::<FileData>);
    let mut is_uploading = use_signal(|| false);
    let mut is_drag_over = use_signal(|| false);
    let mut success_message = use_signal(String::new);
    let mut error_message = use_signal(String::new);

    let handle_pick_file = move |event: Event<FormData>| {
        let mut files = event.files().into_iter();
        match files.next() {
            Some(file) => {
                selected_file.set(Some(file));
                is_drag_over.set(false);
                success_message.set(String::new());
                error_message.set(String::new());
            }
            None => {
                selected_file.set(None);
                is_drag_over.set(false);
            }
        }
    };

    let handle_drag_enter = move |event: dioxus::events::DragEvent| {
        event.prevent_default();
        is_drag_over.set(true);
    };

    let handle_drag_over = move |event: dioxus::events::DragEvent| {
        event.prevent_default();
        is_drag_over.set(true);
    };

    let handle_drag_leave = move |event: dioxus::events::DragEvent| {
        event.prevent_default();
        is_drag_over.set(false);
    };

    let handle_drop = move |event: dioxus::events::DragEvent| {
        event.prevent_default();
        is_drag_over.set(false);

        let mut files = event.data_transfer().files().into_iter();
        match files.next() {
            Some(file) => {
                selected_file.set(Some(file));
                success_message.set(String::new());
                error_message.set(String::new());
            }
            None => {
                error_message.set("未检测到可上传的图片文件".to_string());
            }
        }
    };

    let clipboard_image_service = image_service.clone();
    let clipboard_toast_store = toast_store.clone();
    let handle_clipboard_upload = move |event: dioxus::events::MouseEvent| {
        event.prevent_default();
        if is_uploading() {
            return;
        }

        let image_service = clipboard_image_service.clone();
        let toast_store = clipboard_toast_store.clone();
        spawn(async move {
            is_uploading.set(true);
            error_message.set(String::new());
            success_message.set(String::new());

            let (filename, content_type, bytes) = match read_image_from_clipboard().await {
                Ok(payload) => payload,
                Err(err) => {
                    error_message.set(err.clone());
                    toast_store.show_error(err);
                    is_uploading.set(false);
                    return;
                }
            };

            match image_service
                .upload_image(filename, content_type, bytes)
                .await
            {
                Ok(image) => {
                    let message = format!("剪贴板上传成功: {}", image.filename);
                    success_message.set(message.clone());
                    toast_store.show_success(message);
                    selected_file.set(None);
                }
                Err(err) => {
                    let message = format!("剪贴板上传失败: {}", err);
                    error_message.set(message.clone());
                    toast_store.show_error(message);
                }
            }

            is_uploading.set(false);
        });
    };

    let handle_upload = move |_| {
        if is_uploading() {
            return;
        }

        let Some(file) = selected_file() else {
            let message = "请先选择图片文件".to_string();
            error_message.set(message.clone());
            toast_store.show_error(message);
            return;
        };

        let image_service = image_service.clone();
        let toast_store = toast_store.clone();

        spawn(async move {
            is_uploading.set(true);
            error_message.set(String::new());
            success_message.set(String::new());

            let filename = file.name();
            let content_type = file.content_type();
            let bytes = match file.read_bytes().await {
                Ok(data) => data.to_vec(),
                Err(err) => {
                    let message = format!("读取文件失败: {}", err);
                    error_message.set(message.clone());
                    toast_store.show_error(message);
                    is_uploading.set(false);
                    return;
                }
            };

            if bytes.is_empty() {
                let message = "文件内容为空".to_string();
                error_message.set(message.clone());
                toast_store.show_error(message);
                is_uploading.set(false);
                return;
            }

            match image_service
                .upload_image(filename, content_type, bytes)
                .await
            {
                Ok(image) => {
                    let message = format!("上传成功: {}", image.filename);
                    success_message.set(message.clone());
                    toast_store.show_success(message);
                    selected_file.set(None);
                }
                Err(err) => {
                    let message = format!("上传失败: {}", err);
                    error_message.set(message.clone());
                    toast_store.show_error(message);
                }
            }

            is_uploading.set(false);
        });
    };

    let file_name = selected_file()
        .as_ref()
        .map(|f| f.name())
        .unwrap_or_default();
    let file_size = selected_file()
        .as_ref()
        .map(|f| format_bytes(f.size()))
        .unwrap_or_default();

    rsx! {
        div { class: "dashboard-page upload-page",
            section { class: "upload-scene",
                section {
                    class: format!("upload-card {}", if is_drag_over() { "is-drag-over" } else { "" }),

                    input {
                        id: "upload-file",
                        class: "upload-hidden-input",
                        r#type: "file",
                        accept: "image/*",
                        onchange: handle_pick_file,
                        disabled: is_uploading(),
                    }

                    span { class: "upload-settings-icon", "⚙" }

                    label {
                        class: format!("upload-dropzone {}", if is_drag_over() { "is-drag-over" } else { "" }),
                        for: "upload-file",
                        ondragenter: handle_drag_enter,
                        ondragover: handle_drag_over,
                        ondragleave: handle_drag_leave,
                        ondrop: handle_drop,

                        div { class: "upload-folder-icon" }
                        h2 { class: "upload-drop-title", "点击或拖拽上传图片" }
                        p { class: "upload-drop-desc", "支持 JPG / PNG / WEBP / GIF，单文件最大 100MB" }
                        p { class: "upload-drop-note", "上传后会自动按时间排序展示在历史页面" }
                    }

                    if !file_name.is_empty() {
                        div { class: "upload-file-meta",
                            span { class: "upload-file-name", "{file_name}" }
                            span { class: "upload-file-size", "{file_size}" }
                        }
                    }

                    p { class: "upload-tip-line",
                        span { class: "upload-tip-icon", "💡" }
                        span {
                            "你也可以直接"
                            button {
                                class: format!("upload-tip-link {}", if is_uploading() { "is-disabled" } else { "" }),
                                r#type: "button",
                                onclick: handle_clipboard_upload,
                                "粘贴剪贴板中的图片"
                            }
                        }
                    }

                    div { class: "upload-actions",
                        button {
                            class: "btn btn-primary upload-submit",
                            disabled: is_uploading(),
                            onclick: handle_upload,
                            if is_uploading() {
                                "上传中..."
                            } else {
                                "开始上传"
                            }
                        }
                    }

                    if !success_message().is_empty() {
                        p { class: "upload-message upload-message-success", "{success_message()}" }
                    }
                    if !error_message().is_empty() {
                        p { class: "upload-message upload-message-error", "{error_message()}" }
                    }
                }
            }
        }
    }
}
