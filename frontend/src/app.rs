use crate::app_context::AppContext;
use crate::components::Toast;
use crate::config::Config;
use crate::pages::{
    ApiPage, BootstrapDatabasePage, ImageListPage, InstallWizardPage, LoginPage, SettingsPage,
    UploadPage,
};
use crate::store::DashboardPage;
use crate::types::api::{BootstrapStatusResponse, InstallStatusResponse};
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

const AUTH_BOOTSTRAP_RETRY_DELAYS_MS: [u32; 3] = [0, 600, 1800];
const DEFAULT_APP_DISPLAY_NAME: &str = "Avenrixa";

/// 应用程序入口组件
#[component]
pub fn App() -> Element {
    let app_context = use_context_provider(|| AppContext::new(Config::api_base_url().to_string()));
    let auth_store = use_context_provider(|| app_context.auth_store.clone());
    use_context_provider(|| app_context.image_store.clone());
    let navigation_store = use_context_provider(|| app_context.navigation_store.clone());
    use_context_provider(|| app_context.api_client.clone());
    let auth_service = use_context_provider(|| app_context.auth_service.clone());
    use_context_provider(|| app_context.admin_service.clone());
    use_context_provider(|| app_context.image_service.clone());
    let install_service = use_context_provider(|| app_context.install_service.clone());
    let _settings_service = use_context_provider(|| app_context.settings_service.clone());
    let toast_store = use_context_provider(|| app_context.toast_store.clone());
    let mut site_name = use_signal(String::new);
    let mut bootstrap_status = use_signal(|| None::<BootstrapStatusResponse>);
    let mut install_status = use_signal(|| None::<InstallStatusResponse>);
    let is_boot_ready = use_signal(|| false);
    let mut favicon_version = use_signal(|| 0_u64);

    use_effect(move || {
        sync_browser_branding(display_site_name(&site_name()), favicon_version());
    });

    let _bootstrap = use_future({
        let install_service = install_service.clone();
        let auth_service = auth_service.clone();
        let auth_store = auth_store.clone();
        let toast_store = toast_store.clone();
        let bootstrap_status = bootstrap_status;
        let site_name = site_name;
        let install_status = install_status;
        let boot_ready_signal = is_boot_ready;
        move || {
            let install_service = install_service.clone();
            let auth_service = auth_service.clone();
            let auth_store = auth_store.clone();
            let toast_store = toast_store.clone();
            let mut bootstrap_status = bootstrap_status;
            let mut site_name = site_name;
            let mut install_status = install_status;
            let mut is_boot_ready = boot_ready_signal;
            async move {
                let status = match install_service.get_install_status().await {
                    Ok(status) => status,
                    Err(err) => {
                        toast_store.show_error(format!("初始化安装状态失败: {}", err));
                        is_boot_ready.set(true);
                        return;
                    }
                };

                site_name.set(status.config.site_name.clone());
                install_status.set(Some(status.clone()));

                if !status.installed {
                    let bootstrap = match install_service.get_bootstrap_status().await {
                        Ok(status) => status,
                        Err(err) => {
                            toast_store.show_error(format!("初始化引导状态失败: {}", err));
                            is_boot_ready.set(true);
                            return;
                        }
                    };

                    bootstrap_status.set(Some(bootstrap));
                    auth_store.logout();
                    is_boot_ready.set(true);
                    return;
                }

                let mut last_error = None;

                for delay_ms in AUTH_BOOTSTRAP_RETRY_DELAYS_MS {
                    if delay_ms > 0 {
                        TimeoutFuture::new(delay_ms).await;
                    }

                    match auth_service.get_me().await {
                        Ok(user) => {
                            auth_store.set_user(user);
                            is_boot_ready.set(true);
                            return;
                        }
                        Err(err) if err.should_redirect_login() => {
                            auth_store.logout();
                            is_boot_ready.set(true);
                            return;
                        }
                        Err(err) => last_error = Some(err),
                    }
                }

                if let Some(err) = last_error {
                    toast_store.show_error(format!("初始化登录状态失败: {}", err));
                    eprintln!("初始化认证状态失败: {}", err);
                }

                is_boot_ready.set(true);
            }
        }
    });

    let is_authenticated = auth_store.is_authenticated();
    let user = auth_store.user();
    let is_admin = user
        .as_ref()
        .map(|current| current.role.is_admin())
        .unwrap_or(false);
    let is_bootstrap_mode = bootstrap_status()
        .as_ref()
        .map(|status| status.mode == "bootstrap")
        .unwrap_or(false);
    let is_uninstalled = install_status()
        .as_ref()
        .map(|status| !status.installed)
        .unwrap_or(false);
    let auth_store_for_install = auth_store.clone();
    let navigation_store_for_install = navigation_store.clone();

    rsx! {
        div { id: "app-root", class: "app-shell",
            if !is_bootstrap_mode && !is_uninstalled {
                crate::components::NavBar { site_name: display_site_name(&site_name()).to_string() }
            }
            main { class: "main-content",
                if !is_boot_ready() {
                    section { class: "dashboard-page page-hero",
                        h1 { "正在初始化站点" }
                    }
                } else if let Some(bootstrap) = bootstrap_status() {
                    if bootstrap.mode == "bootstrap" {
                        BootstrapDatabasePage {
                            status: bootstrap.clone(),
                            on_status_updated: move |next: BootstrapStatusResponse| {
                                bootstrap_status.set(Some(next));
                            },
                        }
                    } else if let Some(status) = install_status() {
                        if !status.installed {
                            InstallWizardPage {
                                bootstrap_status: bootstrap.clone(),
                                initial_config: status.config.clone(),
                                on_installed: move |response: crate::types::api::InstallBootstrapResponse| {
                                    site_name.set(response.config.site_name.clone());
                                    auth_store_for_install.set_user(response.user.clone());
                                    navigation_store_for_install.reset();
                                    install_status.set(Some(InstallStatusResponse {
                                        installed: true,
                                        has_admin: true,
                                        favicon_configured: response.favicon_configured,
                                        config: response.config.clone(),
                                    }));
                                    favicon_version.set(js_sys::Date::now() as u64);
                                },
                            }
                        } else if is_authenticated {
                            match navigation_store.current_page() {
                                DashboardPage::Upload => rsx! { UploadPage {} },
                                DashboardPage::History => rsx! { ImageListPage {} },
                                DashboardPage::Api => rsx! { ApiPage {} },
                                DashboardPage::Settings => rsx! {
                                    SettingsPage {
                                        is_admin,
                                        requested_section: navigation_store.current_settings_anchor(),
                                        on_site_name_updated: move |name| site_name.set(name),
                                    }
                                },
                            }
                        } else {
                            LoginPage { mail_enabled: status.config.mail_enabled }
                        }
                    } else {
                        section { class: "dashboard-page page-hero",
                            h1 { "初始化失败" }
                            p { "运行时已就绪，但未能加载安装状态，请刷新页面后重试。" }
                        }
                    }
                } else if let Some(status) = install_status() {
                    if status.installed && is_authenticated {
                        match navigation_store.current_page() {
                            DashboardPage::Upload => rsx! { UploadPage {} },
                            DashboardPage::History => rsx! { ImageListPage {} },
                            DashboardPage::Api => rsx! { ApiPage {} },
                            DashboardPage::Settings => rsx! {
                                SettingsPage {
                                    is_admin,
                                    requested_section: navigation_store.current_settings_anchor(),
                                    on_site_name_updated: move |name| site_name.set(name),
                                }
                            },
                        }
                    } else {
                        LoginPage { mail_enabled: status.config.mail_enabled }
                    }
                } else {
                    section { class: "dashboard-page page-hero",
                        h1 { "初始化失败" }
                        p { "未能加载引导状态，请刷新页面后重试。" }
                    }
                }
            }
            Toast {}
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn sync_browser_branding(site_name: &str, favicon_version: u64) {
    if let Some(window) = web_sys::window()
        && let Some(document) = window.document()
    {
        document.set_title(site_name);
        if let Some(link) = document.get_element_by_id("app-favicon") {
            let href = if favicon_version == 0 {
                "/favicon.ico".to_string()
            } else {
                format!("/favicon.ico?v={}", favicon_version)
            };
            let _ = link.set_attribute("href", &href);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn sync_browser_branding(_site_name: &str, _favicon_version: u64) {}

fn display_site_name(site_name: &str) -> &str {
    let site_name = site_name.trim();
    if site_name.is_empty() {
        DEFAULT_APP_DISPLAY_NAME
    } else {
        site_name
    }
}
