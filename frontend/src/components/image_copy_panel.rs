use crate::app_context::use_toast_store;
use crate::types::models::ImageItem;
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
fn current_origin() -> Option<String> {
    web_sys::window()
        .and_then(|window| window.location().origin().ok())
        .filter(|origin| !origin.trim().is_empty())
}

#[cfg(not(target_arch = "wasm32"))]
fn current_origin() -> Option<String> {
    None
}

pub fn absolute_image_url(image: &ImageItem) -> String {
    let relative = image.url();
    match current_origin() {
        Some(origin) => format!("{origin}{relative}"),
        None => relative,
    }
}

#[cfg(target_arch = "wasm32")]
async fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    use js_sys::{Function, Promise, Reflect};
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or_else(|| "无法访问浏览器窗口".to_string())?;
    let navigator = window.navigator();
    let clipboard = Reflect::get(navigator.as_ref(), &JsValue::from_str("clipboard"))
        .map_err(|_| "无法访问浏览器剪贴板接口".to_string())?;

    if clipboard.is_null() || clipboard.is_undefined() {
        return legacy_copy_text_to_clipboard(&text);
    }

    let write_text = Reflect::get(&clipboard, &JsValue::from_str("writeText"))
        .map_err(|_| "当前浏览器不支持剪贴板复制".to_string())?;

    let Some(write_text_fn) = write_text.dyn_ref::<Function>() else {
        return legacy_copy_text_to_clipboard(&text);
    };

    let promise = write_text_fn
        .call1(&clipboard, &JsValue::from_str(&text))
        .map_err(|_| "复制失败，请确认浏览器已授予剪贴板权限".to_string())?
        .dyn_into::<Promise>()
        .map_err(|_| "复制失败，请确认浏览器已授予剪贴板权限".to_string())?;

    JsFuture::from(promise)
        .await
        .map_err(|_| "复制失败，请确认浏览器已授予剪贴板权限".to_string())?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn legacy_copy_text_to_clipboard(text: &str) -> Result<(), String> {
    use js_sys::{Function, Reflect};
    use wasm_bindgen::{JsCast, JsValue};

    let window = web_sys::window().ok_or_else(|| "无法访问浏览器窗口".to_string())?;
    let document = window
        .document()
        .ok_or_else(|| "无法访问浏览器文档".to_string())?;
    let body = document
        .body()
        .ok_or_else(|| "页面尚未准备好，无法复制".to_string())?;
    let textarea = document
        .create_element("textarea")
        .map_err(|_| "创建复制缓冲区失败".to_string())?;

    textarea
        .set_attribute("readonly", "true")
        .map_err(|_| "初始化复制缓冲区失败".to_string())?;
    textarea
        .set_attribute(
            "style",
            "position:fixed;top:-1000px;left:-1000px;opacity:0;pointer-events:none;",
        )
        .map_err(|_| "初始化复制缓冲区失败".to_string())?;
    Reflect::set(
        textarea.as_ref(),
        &JsValue::from_str("value"),
        &JsValue::from_str(text),
    )
    .map_err(|_| "写入复制内容失败".to_string())?;

    body.append_child(&textarea)
        .map_err(|_| "挂载复制缓冲区失败".to_string())?;

    let result = (|| {
        let select = Reflect::get(textarea.as_ref(), &JsValue::from_str("select"))
            .map_err(|_| "当前浏览器不支持复制".to_string())?;
        let Some(select_fn) = select.dyn_ref::<Function>() else {
            return Err("当前浏览器不支持复制".to_string());
        };
        select_fn
            .call0(textarea.as_ref())
            .map_err(|_| "选中复制内容失败".to_string())?;

        let exec_command = Reflect::get(document.as_ref(), &JsValue::from_str("execCommand"))
            .map_err(|_| "当前浏览器不支持复制".to_string())?;
        let Some(exec_command_fn) = exec_command.dyn_ref::<Function>() else {
            return Err("当前浏览器不支持复制".to_string());
        };
        let copied = exec_command_fn
            .call1(document.as_ref(), &JsValue::from_str("copy"))
            .map_err(|_| "复制失败，请手动复制".to_string())?
            .as_bool()
            .unwrap_or(false);

        if copied {
            Ok(())
        } else {
            Err("复制失败，请手动复制".to_string())
        }
    })();

    let _ = body.remove_child(&textarea);
    result
}

#[cfg(not(target_arch = "wasm32"))]
async fn copy_text_to_clipboard(_text: String) -> Result<(), String> {
    Err("当前运行环境不支持复制到剪贴板".to_string())
}

fn markdown_image_snippet(image: &ImageItem, url: &str) -> String {
    format!("![{}]({url})", image.filename)
}

fn markdown_link_snippet(image: &ImageItem, url: &str) -> String {
    format!("[{}]({url})", image.filename)
}

fn html_image_snippet(image: &ImageItem, url: &str) -> String {
    format!("<img src=\"{url}\" alt=\"{}\" />", image.filename)
}

fn bbcode_image_snippet(url: &str) -> String {
    format!("[img]{url}[/img]")
}

#[component]
fn CopyVariantButton(label: &'static str, text: String) -> Element {
    let toast_store = use_toast_store();

    let handle_copy = move |_| {
        let toast_store = toast_store.clone();
        let text = text.clone();
        spawn(async move {
            match copy_text_to_clipboard(text).await {
                Ok(()) => toast_store.show_success(format!("已复制{label}")),
                Err(err) => toast_store.show_error(err),
            }
        });
    };

    rsx! {
        button {
            class: "btn btn-secondary upload-result-copy",
            r#type: "button",
            onclick: handle_copy,
            "复制{label}"
        }
    }
}

#[component]
pub fn ImageCopyPanel(image: ImageItem) -> Element {
    let direct_url = absolute_image_url(&image);
    let markdown_image = markdown_image_snippet(&image, &direct_url);
    let markdown_link = markdown_link_snippet(&image, &direct_url);
    let html_image = html_image_snippet(&image, &direct_url);
    let bbcode_image = bbcode_image_snippet(&direct_url);

    rsx! {
        div { class: "upload-result-row",
            div { class: "upload-result-copy-main",
                span { class: "upload-result-label", "直连" }
                code { class: "upload-result-code", "{direct_url}" }
            }
            CopyVariantButton {
                label: "直连",
                text: direct_url.clone(),
            }
        }

        div { class: "upload-result-copy-grid",
            CopyVariantButton {
                label: "Markdown 图片",
                text: markdown_image,
            }
            CopyVariantButton {
                label: "Markdown 链接",
                text: markdown_link,
            }
            CopyVariantButton {
                label: "HTML 图片",
                text: html_image,
            }
            CopyVariantButton {
                label: "BBCode",
                text: bbcode_image,
            }
        }
    }
}
