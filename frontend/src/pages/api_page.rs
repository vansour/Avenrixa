use dioxus::prelude::*;

/// API 信息页组件
#[component]
pub fn ApiPage() -> Element {
    rsx! {
        div { class: "dashboard-page api-page",
            section { class: "page-hero",
                h1 { "API 信息" }
                p { "当前前后端使用 Cookie 会话认证，接口前缀为 /api/v1。" }
            }

            section { class: "api-card",
                h2 { "认证方式" }
                p { "登录接口成功后写入 HttpOnly Cookie，后续请求自动携带。"}
                code { "POST /api/v1/auth/login" }
            }

            section { class: "api-card",
                h2 { "核心接口" }
                ul { class: "api-list",
                    li { code { "POST /api/v1/upload" } " - 上传图片（multipart, 字段名 file）" }
                    li { code { "GET /api/v1/images?page=1&page_size=20" } " - 获取历史图片列表（按上传时间倒序）" }
                    li { code { "DELETE /api/v1/images" } " - 批量删除，参数 image_keys" }
                    li { code { "POST /api/v1/images/restore" } " - 批量恢复，参数 image_keys" }
                    li { code { "GET /api/v1/settings/config" } " - 获取管理员结构化设置" }
                    li { code { "PUT /api/v1/settings/config" } " - 更新管理员结构化设置" }
                    li { code { "GET /thumbnails/:hash.webp" } " - 动态缩略图（不落盘）" }
                }
            }
        }
    }
}
