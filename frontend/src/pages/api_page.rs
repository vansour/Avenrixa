use crate::config::Config;
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
fn current_origin() -> Option<String> {
    web_sys::window()
        .and_then(|window| window.location().origin().ok())
        .filter(|origin| !origin.trim().is_empty() && origin != "null")
}

#[cfg(not(target_arch = "wasm32"))]
fn current_origin() -> Option<String> {
    None
}

fn api_base_label() -> String {
    let base = Config::api_base_url().trim();
    if base.is_empty() || base == "/" {
        return current_origin().unwrap_or_else(|| "/".to_string());
    }

    if base.starts_with("http://") || base.starts_with("https://") {
        return base.trim_end_matches('/').to_string();
    }

    match current_origin() {
        Some(origin) => format!(
            "{}{}",
            origin.trim_end_matches('/'),
            if base.starts_with('/') {
                base.trim_end_matches('/').to_string()
            } else {
                format!("/{}", base.trim_end_matches('/'))
            }
        ),
        None => base.to_string(),
    }
}

#[component]
fn ApiSidebarLink(href: &'static str, label: &'static str, detail: &'static str) -> Element {
    rsx! {
        a { class: "api-nav-link", href: "{href}",
            strong { "{label}" }
            if !detail.is_empty() {
                span { "{detail}" }
            }
        }
    }
}

#[component]
fn ApiQuickStat(label: &'static str, value: String) -> Element {
    rsx! {
        article { class: "api-quick-stat",
            span { class: "api-quick-label", "{label}" }
            code { class: "api-quick-value", "{value}" }
        }
    }
}

#[component]
fn ApiEndpointRow(method: &'static str, path: &'static str, detail: &'static str) -> Element {
    rsx! {
        div { class: "api-endpoint-row",
            div { class: "api-endpoint-head",
                span { class: format!("api-method api-method-{}", method.to_ascii_lowercase()), "{method}" }
                code { class: "api-endpoint-path", "{path}" }
            }
            p { class: "api-endpoint-copy", "{detail}" }
        }
    }
}

#[component]
fn ApiExampleCard(title: &'static str, code: String) -> Element {
    rsx! {
        article { class: "api-example-card",
            div { class: "api-example-head",
                p { class: "api-section-kicker", "Example" }
                h3 { "{title}" }
            }
            pre { class: "api-code-block api-code-block-compact",
                code { "{code}" }
            }
        }
    }
}

#[component]
pub fn ApiPage() -> Element {
    let api_base = api_base_label();
    let login_endpoint = format!("{}/api/v1/auth/login", api_base.trim_end_matches('/'));
    let upload_endpoint = format!("{}/api/v1/upload", api_base.trim_end_matches('/'));
    let images_endpoint = format!(
        "{}/api/v1/images?page=1&page_size=20",
        api_base.trim_end_matches('/')
    );
    let media_endpoint = format!("{}/images/{{filename}}", api_base.trim_end_matches('/'));

    let login_curl = format!(
        "curl -X POST '{login_endpoint}' \\\n  -H 'Content-Type: application/json' \\\n  -c cookies.txt \\\n  -d '{{\"email\":\"admin@example.com\",\"password\":\"your-password\"}}'"
    );
    let upload_curl =
        format!("curl -X POST '{upload_endpoint}' \\\n  -b cookies.txt \\\n  -F 'file=@demo.png'");
    let browser_fetch = format!(
        "const form = new FormData();\nform.append('file', fileInput.files[0]);\nconst response = await fetch('{upload_endpoint}', {{\n  method: 'POST',\n  body: form,\n  credentials: 'include',\n}});"
    );

    rsx! {
        div { class: "dashboard-page api-page",
            div { class: "api-shell",
                aside { class: "api-sidebar",
                    div { class: "api-sidebar-card",
                        p { class: "api-sidebar-eyebrow", "API" }
                        h1 { "接入速查" }
                    }

                    nav { class: "api-nav",
                        ApiSidebarLink {
                            href: "#api-overview",
                            label: "概览",
                            detail: ""
                        }
                        ApiSidebarLink {
                            href: "#api-auth",
                            label: "认证",
                            detail: ""
                        }
                        ApiSidebarLink {
                            href: "#api-upload",
                            label: "上传",
                            detail: ""
                        }
                        ApiSidebarLink {
                            href: "#api-images",
                            label: "图片",
                            detail: ""
                        }
                        ApiSidebarLink {
                            href: "#api-media",
                            label: "媒体",
                            detail: ""
                        }
                    }
                }

                main { class: "api-content",
                    section { id: "api-overview", class: "api-card api-section" ,
                        div { class: "api-section-head api-section-head-compact",
                            div {
                                p { class: "api-section-kicker", "Overview" }
                                h2 { "接入约定" }
                            }
                        }

                        div { class: "api-quick-grid",
                            ApiQuickStat { label: "API Base", value: api_base.clone() }
                            ApiQuickStat { label: "认证", value: "HttpOnly Cookie Session".to_string() }
                            ApiQuickStat { label: "上传", value: "multipart/form-data".to_string() }
                            ApiQuickStat { label: "媒体", value: media_endpoint.clone() }
                        }
                    }

                    section { id: "api-auth", class: "api-card api-section",
                        div { class: "api-section-head api-section-head-compact",
                            div {
                                p { class: "api-section-kicker", "Auth" }
                                h2 { "认证" }
                            }
                        }

                        ApiEndpointRow {
                            method: "POST",
                            path: "/api/v1/auth/login",
                            detail: "登录并写入 HttpOnly Cookie。"
                        }
                        ApiEndpointRow {
                            method: "GET",
                            path: "/api/v1/auth/me",
                            detail: "读取当前会话。"
                        }
                        ApiEndpointRow {
                            method: "POST",
                            path: "/api/v1/auth/refresh",
                            detail: "刷新会话。"
                        }

                        div { class: "api-examples-grid",
                            ApiExampleCard {
                                title: "cURL 登录",
                                code: login_curl.clone(),
                            }
                        }
                    }

                    section { id: "api-upload", class: "api-card api-section",
                        div { class: "api-section-head api-section-head-compact",
                            div {
                                p { class: "api-section-kicker", "Upload" }
                                h2 { "上传" }
                            }
                        }

                        ApiEndpointRow {
                            method: "POST",
                            path: "/api/v1/upload",
                            detail: "上传单图，字段名固定为 file。"
                        }

                        div { class: "api-examples-grid",
                            ApiExampleCard {
                                title: "cURL 上传",
                                code: upload_curl.clone(),
                            }
                            ApiExampleCard {
                                title: "Browser fetch",
                                code: browser_fetch.clone(),
                            }
                        }
                    }

                    section { id: "api-images", class: "api-card api-section",
                        div { class: "api-section-head api-section-head-compact",
                            div {
                                p { class: "api-section-kicker", "Images" }
                                h2 { "图片管理" }
                            }
                        }

                        div { class: "api-endpoint-list",
                            ApiEndpointRow {
                                method: "GET",
                                path: "/api/v1/images?page=1&page_size=20",
                                detail: "按时间倒序返回列表。"
                            }
                            ApiEndpointRow {
                                method: "GET",
                                path: "/api/v1/images/{{image_key}}",
                                detail: "读取单图详情。"
                            }
                            ApiEndpointRow {
                                method: "DELETE",
                                path: "/api/v1/images",
                                detail: "批量删除，使用 image_keys 数组。"
                            }
                            ApiEndpointRow {
                                method: "PUT",
                                path: "/api/v1/images/{{image_key}}/expiry",
                                detail: "设置或清空过期时间。"
                            }
                        }

                        div { class: "api-inline-note",
                            span { "列表地址示例" }
                            code { "{images_endpoint}" }
                        }
                    }

                    section { id: "api-media", class: "api-card api-section",
                        div { class: "api-section-head api-section-head-compact",
                            div {
                                p { class: "api-section-kicker", "Media" }
                                h2 { "媒体访问" }
                            }
                        }

                        div { class: "api-endpoint-list",
                            ApiEndpointRow {
                                method: "GET",
                                path: "/images/{{filename}}",
                                detail: "原图地址。"
                            }
                            ApiEndpointRow {
                                method: "GET",
                                path: "/thumbnails/{{image_key}}.webp",
                                detail: "缩略图地址。"
                            }
                        }
                    }
                }
            }
        }
    }
}
