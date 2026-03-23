use crate::app_context::AppContext;
use crate::components::Toast;
use crate::config::Config;
use crate::pages::{
    ApiPage, BootstrapDatabasePage, ImageListPage, InstallWizardPage, LoginPage, SettingsPage,
    UploadPage,
};
use crate::services::{AuthService, InstallService};
use crate::store::{AuthStore, DashboardPage, SettingsAnchor, ToastStore};
use crate::types::api::{BootstrapStatusResponse, InstallBootstrapResponse, InstallStatusResponse};
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

const AUTH_BOOTSTRAP_RETRY_DELAYS_MS: [u32; 3] = [0, 600, 1800];
const DEFAULT_APP_DISPLAY_NAME: &str = "Avenrixa";
const INIT_INSTALL_STATUS_ERROR: &str = "运行时已就绪，但未能加载安装状态，请刷新页面后重试。";
const INIT_BOOTSTRAP_STATUS_ERROR: &str = "未能加载引导状态，请刷新页面后重试。";

enum AppShellState {
    Booting,
    Bootstrap(BootstrapStatusResponse),
    Install {
        bootstrap: BootstrapStatusResponse,
        status: InstallStatusResponse,
    },
    Dashboard,
    Login {
        mail_enabled: bool,
    },
    InitError {
        message: &'static str,
    },
}

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
    let site_name = use_signal(String::new);
    let mut bootstrap_status = use_signal(|| None::<BootstrapStatusResponse>);
    let install_status = use_signal(|| None::<InstallStatusResponse>);
    let is_boot_ready = use_signal(|| false);
    let favicon_version = use_signal(|| 0_u64);

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
            initialize_app_shell(
                install_service.clone(),
                auth_service.clone(),
                auth_store.clone(),
                toast_store.clone(),
                bootstrap_status,
                site_name,
                install_status,
                boot_ready_signal,
            )
        }
    });

    let is_authenticated = auth_store.is_authenticated();
    let user = auth_store.user();
    let is_admin = user
        .as_ref()
        .map(|current| current.role.is_admin())
        .unwrap_or(false);
    let current_page = navigation_store.current_page();
    let requested_section = navigation_store.current_settings_anchor();
    let show_navbar = should_show_navbar(bootstrap_status().as_ref(), install_status().as_ref());
    let shell = resolve_app_shell(
        is_boot_ready(),
        is_authenticated,
        bootstrap_status(),
        install_status(),
    );

    let auth_store_for_install = auth_store.clone();
    let navigation_store_for_install = navigation_store.clone();

    rsx! {
        div { id: "app-root", class: "app-shell",
            if show_navbar {
                crate::components::NavBar { site_name: display_site_name(&site_name()).to_string() }
            }
            main { class: "main-content",
                {
                    match shell {
                        AppShellState::Booting => render_booting_shell(),
                        AppShellState::Bootstrap(bootstrap) => rsx! {
                            BootstrapDatabasePage {
                                status: bootstrap,
                                on_status_updated: move |next: BootstrapStatusResponse| {
                                    bootstrap_status.set(Some(next));
                                },
                            }
                        },
                        AppShellState::Install { bootstrap, status } => rsx! {
                            InstallWizardPage {
                                bootstrap_status: bootstrap,
                                initial_config: status.config.clone(),
                                on_installed: move |response: InstallBootstrapResponse| {
                                    handle_install_completed(
                                        response,
                                        auth_store_for_install.clone(),
                                        navigation_store_for_install.clone(),
                                        site_name,
                                        install_status,
                                        favicon_version,
                                    );
                                },
                            }
                        },
                        AppShellState::Dashboard => {
                            render_dashboard_page(
                                current_page,
                                is_admin,
                                requested_section,
                                site_name,
                            )
                        }
                        AppShellState::Login { mail_enabled } => rsx! {
                            LoginPage { mail_enabled }
                        },
                        AppShellState::InitError { message } => render_init_error(message),
                    }
                }
            }
            Toast {}
        }
    }
}

async fn initialize_app_shell(
    install_service: InstallService,
    auth_service: AuthService,
    auth_store: AuthStore,
    toast_store: ToastStore,
    mut bootstrap_status: Signal<Option<BootstrapStatusResponse>>,
    mut site_name: Signal<String>,
    mut install_status: Signal<Option<InstallStatusResponse>>,
    mut is_boot_ready: Signal<bool>,
) {
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

fn handle_install_completed(
    response: InstallBootstrapResponse,
    auth_store: AuthStore,
    navigation_store: crate::store::NavigationStore,
    mut site_name: Signal<String>,
    mut install_status: Signal<Option<InstallStatusResponse>>,
    mut favicon_version: Signal<u64>,
) {
    site_name.set(response.config.site_name.clone());
    auth_store.set_user(response.user.clone());
    navigation_store.reset();
    install_status.set(Some(InstallStatusResponse {
        installed: true,
        has_admin: true,
        favicon_configured: response.favicon_configured,
        config: response.config.clone(),
    }));
    favicon_version.set(js_sys::Date::now() as u64);
}

fn resolve_app_shell(
    is_boot_ready: bool,
    is_authenticated: bool,
    bootstrap_status: Option<BootstrapStatusResponse>,
    install_status: Option<InstallStatusResponse>,
) -> AppShellState {
    if !is_boot_ready {
        return AppShellState::Booting;
    }

    match (bootstrap_status, install_status) {
        (Some(bootstrap), _) if bootstrap.mode == "bootstrap" => {
            AppShellState::Bootstrap(bootstrap)
        }
        (Some(bootstrap), Some(status)) if !status.installed => {
            AppShellState::Install { bootstrap, status }
        }
        (Some(_), Some(status)) | (None, Some(status)) => {
            if status.installed && is_authenticated {
                AppShellState::Dashboard
            } else {
                AppShellState::Login {
                    mail_enabled: status.config.mail_enabled,
                }
            }
        }
        (Some(_), None) => AppShellState::InitError {
            message: INIT_INSTALL_STATUS_ERROR,
        },
        (None, None) => AppShellState::InitError {
            message: INIT_BOOTSTRAP_STATUS_ERROR,
        },
    }
}

fn should_show_navbar(
    bootstrap_status: Option<&BootstrapStatusResponse>,
    install_status: Option<&InstallStatusResponse>,
) -> bool {
    !bootstrap_status.is_some_and(|status| status.mode == "bootstrap")
        && !install_status.is_some_and(|status| !status.installed)
}

fn render_booting_shell() -> Element {
    rsx! {
        section { class: "dashboard-page page-hero",
            h1 { "正在初始化站点" }
        }
    }
}

fn render_dashboard_page(
    page: DashboardPage,
    is_admin: bool,
    requested_section: Option<SettingsAnchor>,
    mut site_name: Signal<String>,
) -> Element {
    match page {
        DashboardPage::Upload => rsx! { UploadPage {} },
        DashboardPage::History => rsx! { ImageListPage {} },
        DashboardPage::Api => rsx! { ApiPage {} },
        DashboardPage::Settings => rsx! {
            SettingsPage {
                is_admin,
                requested_section,
                on_site_name_updated: move |name| site_name.set(name),
            }
        },
    }
}

fn render_init_error(message: &'static str) -> Element {
    rsx! {
        section { class: "dashboard-page page-hero",
            h1 { "初始化失败" }
            p { "{message}" }
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
