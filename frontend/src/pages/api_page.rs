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
            span { "{detail}" }
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
            span { class: format!("api-method api-method-{}", method.to_ascii_lowercase()), "{method}" }
            code { class: "api-endpoint-path", "{path}" }
            p { class: "api-endpoint-copy", "{detail}" }
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
        "const form = new FormData();\nform.append('file', fileInput.files[0]);\n\nconst response = await fetch('{upload_endpoint}', {{\n  method: 'POST',\n  body: form,\n  credentials: 'include',\n}});"
    );

    rsx! {
        div { class: "dashboard-page api-page",
            div { class: "api-shell",
                aside { class: "api-sidebar",
                    div { class: "api-sidebar-card",
                        p { class: "api-sidebar-eyebrow", "API" }
                        h1 { "接入指南" }
                        p { class: "api-sidebar-copy",
                            "保留登录、上传、列表和媒体访问四块核心信息，够前端和脚本接入直接开工。"
                        }
                    }

                    nav { class: "api-nav",
                        ApiSidebarLink {
                            href: "#api-overview",
                            label: "概览",
                            detail: "Base URL 与认证方式"
                        }
                        ApiSidebarLink {
                            href: "#api-auth",
                            label: "认证",
                            detail: "登录获取 Cookie 会话"
                        }
                        ApiSidebarLink {
                            href: "#api-upload",
                            label: "上传",
                            detail: "multipart/form-data"
                        }
                        ApiSidebarLink {
                            href: "#api-images",
                            label: "图片接口",
                            detail: "列表、详情、删除"
                        }
                        ApiSidebarLink {
                            href: "#api-media",
                            label: "媒体地址",
                            detail: "原图与缩略图访问"
                        }
                    }
                }

                main { class: "api-content",
                    section { id: "api-overview", class: "api-card api-section" ,
                        div { class: "api-section-head api-section-head-compact",
                            div {
                                p { class: "api-section-kicker", "Overview" }
                                h2 { "先记住这 4 个约定" }
                            }
                            p {
                                "接口默认用 Cookie 会话认证。受保护接口基本都在 "
                                code { "/api/v1" }
                                " 下，原图与缩略图走根路径。"
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
                                h2 { "1. 先登录，拿到 Cookie 会话" }
                            }
                            p { "脚本侧把 Cookie 持久化到文件即可；浏览器侧请求记得带上 credentials: 'include'。" }
                        }

                        ApiEndpointRow {
                            method: "POST",
                            path: "/api/v1/auth/login",
                            detail: "邮箱 + 密码登录，成功后服务端写入 HttpOnly Cookie。"
                        }
                        ApiEndpointRow {
                            method: "GET",
                            path: "/api/v1/auth/me",
                            detail: "读取当前登录用户，适合页面启动时恢复登录态。"
                        }
                        ApiEndpointRow {
                            method: "POST",
                            path: "/api/v1/auth/refresh",
                            detail: "刷新会话；前端部分 401 场景会自动尝试一次。"
                        }

                        pre { class: "api-code-block",
                            code { "{login_curl}" }
                        }
                    }

                    section { id: "api-upload", class: "api-card api-section",
                        div { class: "api-section-head api-section-head-compact",
                            div {
                                p { class: "api-section-kicker", "Upload" }
                                h2 { "2. 上传接口只认 file 字段" }
                            }
                            p { "不要手动设置 multipart 的 Content-Type，交给浏览器或客户端自动带 boundary。" }
                        }

                        ApiEndpointRow {
                            method: "POST",
                            path: "/api/v1/upload",
                            detail: "上传单张图片，字段名固定为 file，返回图片元信息。"
                        }

                        div { class: "api-example-stack",
                            pre { class: "api-code-block",
                                code { "{upload_curl}" }
                            }
                            pre { class: "api-code-block",
                                code { "{browser_fetch}" }
                            }
                        }
                    }

                    section { id: "api-images", class: "api-card api-section",
                        div { class: "api-section-head api-section-head-compact",
                            div {
                                p { class: "api-section-kicker", "Images" }
                                h2 { "3. 图片管理就看这 4 个接口" }
                            }
                            p { "列表页、上传历史、删除动作基本都围绕这组接口展开。" }
                        }

                        div { class: "api-endpoint-list",
                            ApiEndpointRow {
                                method: "GET",
                                path: "/api/v1/images?page=1&page_size=20",
                                detail: "按上传时间倒序返回图片列表。"
                            }
                            ApiEndpointRow {
                                method: "GET",
                                path: "/api/v1/images/{{image_key}}",
                                detail: "获取单张图片详情。"
                            }
                            ApiEndpointRow {
                                method: "DELETE",
                                path: "/api/v1/images",
                                detail: "批量永久删除，请求体使用 image_keys 数组。"
                            }
                            ApiEndpointRow {
                                method: "PUT",
                                path: "/api/v1/images/{{image_key}}/expiry",
                                detail: "设置或清空单张图片的过期时间。"
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
                                h2 { "4. 原图和缩略图不在 /api/v1 下" }
                            }
                            p { "上传成功后的 filename 和 image_key 可以直接拼媒体访问地址。" }
                        }

                        div { class: "api-endpoint-list",
                            ApiEndpointRow {
                                method: "GET",
                                path: "/images/{{filename}}",
                                detail: "原图直链。"
                            }
                            ApiEndpointRow {
                                method: "GET",
                                path: "/thumbnails/{{image_key}}.webp",
                                detail: "缩略图地址，适合图库和历史页预览。"
                            }
                        }

                        div { class: "api-callout" ,
                            strong { "接入建议" }
                            p {
                                "浏览器项目优先复用站点会话；脚本项目优先走登录 + Cookie 文件。这样不需要单独维护 Bearer Token。"
                            }
                        }
                    }
                }
            }
        }
    }
}
