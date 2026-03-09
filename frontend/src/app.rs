use crate::app_context::AppContext;
use crate::components::{NavBar, Toast, TopPage};
use crate::config::Config;
use crate::pages::{
    ApiPage, DeletedImagesPage, ImageListPage, LoginPage, SettingsPage, UploadPage,
};
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

const AUTH_BOOTSTRAP_RETRY_DELAYS_MS: [u32; 3] = [0, 600, 1800];

/// 应用程序入口组件
#[component]
pub fn App() -> Element {
    let app_context = use_context_provider(|| AppContext::new(Config::api_base_url().to_string()));
    let auth_store = use_context_provider(|| app_context.auth_store.clone());
    use_context_provider(|| app_context.image_store.clone());
    use_context_provider(|| app_context.api_client.clone());
    let auth_service = use_context_provider(|| app_context.auth_service.clone());
    use_context_provider(|| app_context.admin_service.clone());
    use_context_provider(|| app_context.image_service.clone());
    let settings_service = use_context_provider(|| app_context.settings_service.clone());
    let toast_store = use_context_provider(|| app_context.toast_store.clone());
    let mut current_page = use_signal(|| TopPage::Upload);
    let mut site_name = use_signal(|| "Vansour Image".to_string());
    let is_auth_ready = use_signal(|| false);

    let _init_auth = use_future({
        let auth_service = auth_service.clone();
        let auth_store = auth_store.clone();
        let toast_store = toast_store.clone();
        let is_auth_ready = is_auth_ready;
        move || {
            let auth_service = auth_service.clone();
            let auth_store = auth_store.clone();
            let toast_store = toast_store.clone();
            let mut is_auth_ready = is_auth_ready;
            async move {
                let mut last_error = None;

                for delay_ms in AUTH_BOOTSTRAP_RETRY_DELAYS_MS {
                    if delay_ms > 0 {
                        TimeoutFuture::new(delay_ms).await;
                    }

                    match auth_service.get_me().await {
                        Ok(user) => {
                            auth_store.set_user(user);
                            is_auth_ready.set(true);
                            return;
                        }
                        Err(err) if err.should_redirect_login() => {
                            auth_store.logout();
                            is_auth_ready.set(true);
                            return;
                        }
                        Err(err) => last_error = Some(err),
                    }
                }

                if let Some(err) = last_error {
                    toast_store.show_error(format!("初始化登录状态失败: {}", err));
                    eprintln!("初始化认证状态失败: {}", err);
                }

                is_auth_ready.set(true);
            }
        }
    });

    let is_authenticated = auth_store.is_authenticated();
    let user = auth_store.user();
    let is_admin = user
        .as_ref()
        .map(|current| current.role.eq_ignore_ascii_case("admin"))
        .unwrap_or(false);

    let _load_site_name = use_resource({
        let settings_service = settings_service.clone();
        move || {
            let auth_user = auth_store.user();
            let settings_service = settings_service.clone();
            async move {
                if let Some(user) = auth_user
                    && user.role.eq_ignore_ascii_case("admin")
                    && let Ok(config) = settings_service.get_admin_settings_config().await
                {
                    site_name.set(config.site_name);
                    return;
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
                on_navigate: move |page| current_page.set(page)
            }
            main { class: "main-content",
                if !is_auth_ready() {
                    section { class: "dashboard-page page-hero",
                        h1 { "正在恢复登录状态" }
                    }
                } else if is_authenticated {
                    match current_page() {
                        TopPage::Upload => rsx! { UploadPage {} },
                        TopPage::History => rsx! { ImageListPage {} },
                        TopPage::Trash => rsx! { DeletedImagesPage {} },
                        TopPage::Api => rsx! { ApiPage {} },
                        TopPage::Settings => rsx! {
                            SettingsPage {
                                is_admin,
                                on_site_name_updated: move |name| site_name.set(name),
                            }
                        },
                    }
                } else {
                    LoginPage {}
                }
            }
            Toast {}
        }
    }
}
