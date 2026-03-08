use crate::app_context::AppContext;
use crate::config::Config;
use dioxus::prelude::*;

/// 应用程序入口组件
#[component]
pub fn App() -> Element {
    let app_context = AppContext::new(Config::api_base_url().to_string());

    // 提供应用上下文给整个应用
    provide_context(app_context.clone());

    // 根据认证状态显示不同内容
    if app_context.auth_store.is_authenticated() {
        rsx! {
            Link { to: "/images", "我的图片" }
        }
    } else {
        rsx! {
            Link { to: "/login", "登录" }
        }
    }
}
