use crate::app_context::AppContext;
use crate::components::{NavBar, Toast};
use crate::config::Config;
use crate::pages::{ImageListPage, LoginPage};
use dioxus::prelude::*;

/// 应用程序入口组件
#[component]
pub fn App() -> Element {
    // 仅初始化一次，避免每次渲染重建全局状态
    let app_context = use_context_provider(|| AppContext::new(Config::api_base_url().to_string()));
    let auth_store = use_context_provider(|| app_context.auth_store.clone());
    use_context_provider(|| app_context.image_store.clone());
    use_context_provider(|| app_context.api_client.clone());
    let auth_service = use_context_provider(|| app_context.auth_service.clone());
    use_context_provider(|| app_context.image_service.clone());
    let toast_store = use_context_provider(|| app_context.toast_store.clone());

    // 应用启动时尝试恢复会话
    let _init_auth = use_future({
        let auth_service = auth_service.clone();
        let auth_store = auth_store.clone();
        let toast_store = toast_store.clone();
        move || {
            let auth_service = auth_service.clone();
            let auth_store = auth_store.clone();
            let toast_store = toast_store.clone();
            async move {
                match auth_service.get_me().await {
                    Ok(user) => auth_store.set_user(user),
                    Err(err) => {
                        if err.should_redirect_login() {
                            auth_store.logout();
                        } else {
                            toast_store.show_error(format!("初始化登录状态失败: {}", err));
                            eprintln!("初始化认证状态失败: {}", err);
                        }
                    }
                }
            }
        }
    });

    let is_authenticated = auth_store.is_authenticated();

    rsx! {
        div { id: "app-root",
            NavBar {}
            Toast {}
            div { class: "main-content",
                if is_authenticated {
                    ImageListPage {}
                } else {
                    LoginPage {}
                }
            }
        }
    }
}
