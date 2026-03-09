use crate::app_context::AppContext;
use crate::components::{NavBar, Toast, TopPage};
use crate::config::Config;
use crate::pages::{ApiPage, ImageListPage, LoginPage, SettingsPage, UploadPage};
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
    let settings_service = use_context_provider(|| app_context.settings_service.clone());
    let toast_store = use_context_provider(|| app_context.toast_store.clone());
    let mut current_page = use_signal(|| TopPage::Upload);
    let mut site_name = use_signal(|| "Vansour Image".to_string());

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
    let user = auth_store.user();
    let is_admin = user
        .as_ref()
        .map(|u| u.role.eq_ignore_ascii_case("admin"))
        .unwrap_or(false);

    let _load_site_name = use_resource({
        let settings_service = settings_service.clone();
        move || {
            let auth_user = auth_store.user();
            let settings_service = settings_service.clone();
            async move {
                if let Some(user) = auth_user
                    && user.role.eq_ignore_ascii_case("admin")
                {
                    if let Ok(config) = settings_service.get_admin_settings_config().await {
                        site_name.set(config.site_name);
                        return;
                    }
                }
                site_name.set("Vansour Image".to_string());
            }
        }
    });

    rsx! {
        div { id: "app-root", class: "app-shell",
            NavBar {
                site_name: site_name(),
                current_page: current_page(),
                on_navigate: move |page| {
                    if page == TopPage::Settings && !is_admin {
                        current_page.set(TopPage::Upload);
                    } else {
                        current_page.set(page);
                    }
                },
            }
            main { class: "main-content",
                if is_authenticated {
                    match current_page() {
                        TopPage::Upload => rsx! { UploadPage {} },
                        TopPage::History => rsx! { ImageListPage {} },
                        TopPage::Api => rsx! { ApiPage {} },
                        TopPage::Settings => {
                            if is_admin {
                                rsx! {
                                    SettingsPage {
                                        on_site_name_updated: move |name| site_name.set(name),
                                    }
                                }
                            } else {
                                rsx! { UploadPage {} }
                            }
                        }
                    }
                } else {
                    LoginPage {}
                }
            }
            Toast {}
        }
    }
}
