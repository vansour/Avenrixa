use crate::app_context::{use_auth_store, use_image_service, use_toast_store};
use crate::auth_session::{auth_session_expired_message, handle_auth_session_error};
use crate::components::ImageCopyPanel;
use crate::types::models::ImageItem;
use dioxus::html::FileData;
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use dioxus::web::WebEventExt;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

const UPLOAD_PASTE_TARGET_ID: &str = "upload-paste-target";

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
async fn read_image_from_web_file(
    file: web_sys::File,
) -> Result<(String, Option<String>, Vec<u8>), String> {
    use js_sys::{Date, Uint8Array};
    use wasm_bindgen_futures::JsFuture;

    let blob: web_sys::Blob = file.clone().unchecked_into();
    let mime = blob.type_();
    let buffer_js = JsFuture::from(blob.array_buffer())
        .await
        .map_err(|_| "读取剪贴板图片内容失败".to_string())?;
    let buffer = Uint8Array::new(&buffer_js);
    let mut bytes = vec![0_u8; buffer.length() as usize];
    buffer.copy_to(&mut bytes);

    if bytes.is_empty() {
        return Err("剪贴板图片内容为空".to_string());
    }

    let filename = if file.name().trim().is_empty() {
        format!(
            "paste-{}.{}",
            Date::now() as u64,
            extension_from_mime(&mime)
        )
    } else {
        file.name()
    };
    let content_type = if mime.trim().is_empty() {
        None
    } else {
        Some(mime)
    };

    Ok((filename, content_type, bytes))
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

#[cfg(target_arch = "wasm32")]
fn first_pasted_image_file(event: &web_sys::ClipboardEvent) -> Result<web_sys::File, String> {
    let Some(data) = event.clipboard_data() else {
        return Err("浏览器没有提供剪贴板数据".to_string());
    };
    let Some(files) = data.files() else {
        return Err("剪贴板中没有可上传的图片".to_string());
    };

    for index in 0..files.length() {
        let Some(file) = files.get(index) else {
            continue;
        };
        let blob: web_sys::Blob = file.clone().unchecked_into();
        if blob.type_().starts_with("image/") {
            return Ok(file);
        }
    }

    Err("剪贴板中没有可上传的图片".to_string())
}

#[cfg(target_arch = "wasm32")]
fn focus_upload_paste_target() {
    if let Some(document) = web_sys::window().and_then(|window| window.document())
        && let Some(element) = document.get_element_by_id(UPLOAD_PASTE_TARGET_ID)
        && let Some(target) = element.dyn_ref::<web_sys::HtmlElement>()
    {
        let _ = target.focus();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn focus_upload_paste_target() {}

fn push_uploaded_image(images: &mut Vec<ImageItem>, image: ImageItem) {
    images.retain(|existing| existing.image_key != image.image_key);
    images.insert(0, image);
}

#[component]
fn UploadResultCard(image: ImageItem) -> Element {
    let preview_url = image.url();
    let thumbnail_url = image.thumbnail_url();
    let display_name = image.display_name();
    let format_label = image.format.to_uppercase();
    let size_label = image.size_formatted();
    let created_at_label = image.created_at_label();

    rsx! {
        section { class: "upload-result-card",
            a {
                class: "upload-result-preview",
                href: "{preview_url}",
                target: "_blank",
                rel: "noreferrer",
                img {
                    src: "{thumbnail_url}",
                    alt: "{display_name}",
                    loading: "lazy"
                }
            }

            div { class: "upload-result-head",
                div {
                    p { class: "upload-result-eyebrow", "本次上传" }
                    h3 { class: "upload-result-title", "{display_name}" }
                }
                a {
                    class: "upload-result-link",
                    href: "{preview_url}",
                    target: "_blank",
                    rel: "noreferrer",
                    "查看原图"
                }
            }

            div { class: "upload-result-main",
                div { class: "upload-result-meta",
                    span { class: "upload-result-chip", "{format_label}" }
                    span { class: "upload-result-chip", "{size_label}" }
                    span { class: "upload-result-chip", "{created_at_label}" }
                }

                ImageCopyPanel { image: image.clone() }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn submit_upload(
    image_service: crate::services::ImageService,
    auth_store: crate::store::auth::AuthStore,
    toast_store: crate::store::toast_store::ToastStore,
    mut selected_file: Signal<Option<FileData>>,
    mut uploaded_images: Signal<Vec<ImageItem>>,
    mut success_message: Signal<String>,
    mut error_message: Signal<String>,
    filename: String,
    content_type: Option<String>,
    bytes: Vec<u8>,
    success_prefix: &'static str,
    failure_prefix: &'static str,
) {
    match image_service
        .upload_image(filename, content_type, bytes)
        .await
    {
        Ok(image) => {
            let message = format!("{success_prefix}: {}", image.filename);
            uploaded_images.with_mut(|images| push_uploaded_image(images, image.clone()));
            success_message.set(message.clone());
            toast_store.show_success(message);
            selected_file.set(None);
        }
        Err(err) => {
            if handle_auth_session_error(&auth_store, &toast_store, &err) {
                error_message.set(auth_session_expired_message());
            } else {
                let message = format!("{failure_prefix}: {}", err);
                error_message.set(message.clone());
                toast_store.show_error(message);
            }
        }
    }
}

/// 上传页面组件
#[component]
pub fn UploadPage() -> Element {
    let auth_store = use_auth_store();
    let image_service = use_image_service();
    let toast_store = use_toast_store();

    let mut selected_file = use_signal(|| None::<FileData>);
    let mut is_uploading = use_signal(|| false);
    let mut is_drag_over = use_signal(|| false);
    let uploaded_images = use_signal(Vec::<ImageItem>::new);
    let mut success_message = use_signal(String::new);
    let mut error_message = use_signal(String::new);

    use_effect(move || {
        focus_upload_paste_target();
    });

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
    let clipboard_auth_store = auth_store.clone();
    let clipboard_toast_store = toast_store.clone();
    #[cfg(target_arch = "wasm32")]
    let paste_image_service = clipboard_image_service.clone();
    #[cfg(target_arch = "wasm32")]
    let paste_auth_store = clipboard_auth_store.clone();
    #[cfg(target_arch = "wasm32")]
    let paste_toast_store = clipboard_toast_store.clone();
    let handle_paste = move |event: dioxus::events::ClipboardEvent| {
        if is_uploading() {
            return;
        }

        #[cfg(target_arch = "wasm32")]
        {
            event.prevent_default();

            let data = event.data();
            let Some(web_event): Option<web_sys::Event> = data.try_as_web_event() else {
                return;
            };
            let Ok(clipboard_event) = web_event.dyn_into::<web_sys::ClipboardEvent>() else {
                return;
            };
            let pasted_file = match first_pasted_image_file(&clipboard_event) {
                Ok(file) => file,
                Err(message) => {
                    error_message.set(message.clone());
                    return;
                }
            };

            let image_service = paste_image_service.clone();
            let auth_store = paste_auth_store.clone();
            let toast_store = paste_toast_store.clone();
            spawn(async move {
                is_uploading.set(true);
                error_message.set(String::new());
                success_message.set(String::new());

                let (filename, content_type, bytes) =
                    match read_image_from_web_file(pasted_file).await {
                        Ok(payload) => payload,
                        Err(err) => {
                            error_message.set(err.clone());
                            toast_store.show_error(err);
                            is_uploading.set(false);
                            return;
                        }
                    };

                submit_upload(
                    image_service,
                    auth_store,
                    toast_store,
                    selected_file,
                    uploaded_images,
                    success_message,
                    error_message,
                    filename,
                    content_type,
                    bytes,
                    "剪贴板上传成功",
                    "剪贴板上传失败",
                )
                .await;

                is_uploading.set(false);
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = event;
        }
    };
    let handle_clipboard_upload = move |event: dioxus::events::MouseEvent| {
        event.prevent_default();
        if is_uploading() {
            return;
        }

        let image_service = clipboard_image_service.clone();
        let auth_store = clipboard_auth_store.clone();
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

            submit_upload(
                image_service,
                auth_store,
                toast_store,
                selected_file,
                uploaded_images,
                success_message,
                error_message,
                filename,
                content_type,
                bytes,
                "剪贴板上传成功",
                "剪贴板上传失败",
            )
            .await;

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
        let auth_store = auth_store.clone();
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

            submit_upload(
                image_service,
                auth_store,
                toast_store,
                selected_file,
                uploaded_images,
                success_message,
                error_message,
                filename,
                content_type,
                bytes,
                "上传成功",
                "上传失败",
            )
            .await;

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
                    id: UPLOAD_PASTE_TARGET_ID,
                    class: format!("upload-card {}", if is_drag_over() { "is-drag-over" } else { "" }),
                    tabindex: "0",
                    onpaste: handle_paste,

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
                        h2 { class: "upload-drop-title", "点击、拖拽或粘贴上传图片" }
                        p { class: "upload-drop-desc", "支持 JPG / PNG / WEBP / GIF，单文件最大 100MB，也支持直接按 Ctrl+V" }
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
                            "你也可以直接按 Ctrl+V，或"
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

            if !uploaded_images().is_empty() {
                section { class: "upload-results-section",
                    div { class: "upload-results-head",
                        h2 { class: "upload-results-title", "本次上传" }
                        p { class: "upload-results-count", "共 {uploaded_images().len()} 张" }
                    }

                    div { class: "upload-results-grid",
                        {uploaded_images().into_iter().map(|image| rsx! {
                            UploadResultCard {
                                key: "{image.image_key}",
                                image
                            }
                        })}
                    }
                }
            }
        }
    }
}
