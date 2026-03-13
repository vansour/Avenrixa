use crate::app_context::AppContext;
use crate::components::{Modal, Toast};
use crate::config::Config;
use crate::pages::{
    ApiPage, BootstrapDatabasePage, DeletedImagesPage, ImageListPage, InstallWizardPage, LoginPage,
    SettingsPage, UploadPage,
};
use crate::store::{DashboardPage, SettingsAnchor};
use crate::types::api::{BootstrapStatusResponse, InstallStatusResponse};
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

const AUTH_BOOTSTRAP_RETRY_DELAYS_MS: [u32; 3] = [0, 600, 1800];

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
    let mut site_name = use_signal(|| "Vansour Image".to_string());
    let mut bootstrap_status = use_signal(|| None::<BootstrapStatusResponse>);
    let mut install_status = use_signal(|| None::<InstallStatusResponse>);
    let is_boot_ready = use_signal(|| false);
    let mut favicon_version = use_signal(|| 0_u64);
    let mut show_first_run_guide = use_signal(|| false);

    use_effect(move || {
        sync_browser_branding(&site_name(), favicon_version());
    });

    use_effect({
        let auth_store = auth_store.clone();
        let mut show_first_run_guide = show_first_run_guide;
        move || {
            let current_user = auth_store.user();
            let current_install_status = install_status();
            let boot_ready = is_boot_ready();

            let eligible = boot_ready
                && current_install_status
                    .as_ref()
                    .is_some_and(|status| status.installed)
                && current_user
                    .as_ref()
                    .is_some_and(|user| user.role.eq_ignore_ascii_case("admin"));

            if let Some(user) = current_user {
                if eligible {
                    show_first_run_guide.set(!is_first_run_guide_dismissed(&user.email));
                } else {
                    show_first_run_guide.set(false);
                }
            } else {
                show_first_run_guide.set(false);
            }
        }
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
        .map(|current| current.role.eq_ignore_ascii_case("admin"))
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
    let auth_store_for_guide_settings = auth_store.clone();
    let navigation_store_for_guide_settings = navigation_store.clone();
    let auth_store_for_guide_storage = auth_store.clone();
    let navigation_store_for_guide_storage = navigation_store.clone();
    let auth_store_for_guide_upload = auth_store.clone();
    let navigation_store_for_guide_upload = navigation_store.clone();
    let auth_store_for_guide_audit = auth_store.clone();
    let navigation_store_for_guide_audit = navigation_store.clone();
    let auth_store_for_guide_close = auth_store.clone();

    rsx! {
        div { id: "app-root", class: "app-shell",
            if !is_bootstrap_mode && !is_uninstalled {
                crate::components::NavBar { site_name: site_name() }
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
                                    reset_first_run_guide_dismissal(&response.user.email);
                                    install_status.set(Some(InstallStatusResponse {
                                        installed: true,
                                        has_admin: true,
                                        favicon_configured: response.favicon_configured,
                                        config: response.config.clone(),
                                    }));
                                    favicon_version.set(js_sys::Date::now() as u64);
                                    show_first_run_guide.set(true);
                                },
                            }
                        } else if is_authenticated {
                            match navigation_store.current_page() {
                                DashboardPage::Upload => rsx! { UploadPage {} },
                                DashboardPage::History => rsx! { ImageListPage {} },
                                DashboardPage::Trash => rsx! { DeletedImagesPage {} },
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
                            DashboardPage::Trash => rsx! { DeletedImagesPage {} },
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
            if show_first_run_guide()
                && is_authenticated
                && is_admin
                && install_status().as_ref().is_some_and(|status| status.installed)
            {
                FirstRunGuideModal {
                    site_name: site_name(),
                    storage_backend: install_status()
                        .as_ref()
                        .map(|status| status.config.storage_backend.clone())
                        .unwrap_or_else(|| "local".to_string()),
                    mail_enabled: install_status()
                        .as_ref()
                        .map(|status| status.config.mail_enabled)
                        .unwrap_or(false),
                    favicon_configured: install_status()
                        .as_ref()
                        .map(|status| status.favicon_configured)
                        .unwrap_or(false),
                    on_go_settings: move |_| {
                        if let Some(user) = auth_store_for_guide_settings.user() {
                            dismiss_first_run_guide(&user.email);
                        }
                        navigation_store_for_guide_settings.open_settings(SettingsAnchor::General);
                        show_first_run_guide.set(false);
                    },
                    on_go_storage: move |_| {
                        if let Some(user) = auth_store_for_guide_storage.user() {
                            dismiss_first_run_guide(&user.email);
                        }
                        navigation_store_for_guide_storage.open_settings(SettingsAnchor::Storage);
                        show_first_run_guide.set(false);
                    },
                    on_go_upload: move |_| {
                        if let Some(user) = auth_store_for_guide_upload.user() {
                            dismiss_first_run_guide(&user.email);
                        }
                        navigation_store_for_guide_upload.navigate(DashboardPage::Upload);
                        show_first_run_guide.set(false);
                    },
                    on_go_audit: move |_| {
                        if let Some(user) = auth_store_for_guide_audit.user() {
                            dismiss_first_run_guide(&user.email);
                        }
                        navigation_store_for_guide_audit.open_settings(SettingsAnchor::Audit);
                        show_first_run_guide.set(false);
                    },
                    on_close: move |_| {
                        if let Some(user) = auth_store_for_guide_close.user() {
                            dismiss_first_run_guide(&user.email);
                        }
                        show_first_run_guide.set(false);
                    },
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

#[component]
fn FirstRunGuideModal(
    site_name: String,
    storage_backend: String,
    mail_enabled: bool,
    favicon_configured: bool,
    on_go_settings: EventHandler<MouseEvent>,
    on_go_storage: EventHandler<MouseEvent>,
    on_go_upload: EventHandler<MouseEvent>,
    on_go_audit: EventHandler<MouseEvent>,
    on_close: EventHandler<()>,
) -> Element {
    let storage_label = if storage_backend.eq_ignore_ascii_case("s3") {
        "S3 / MinIO"
    } else {
        "本地目录"
    };

    rsx! {
        Modal {
            title: "首次进入引导".to_string(),
            on_close,
            div { class: "guide-modal",
                p { class: "settings-eyebrow", "Admin Onboarding" }
                p { class: "guide-intro",
                    "站点 {site_name} 已完成安装。建议先做一次最小验收，确认公开注册、邮件投递和上传链路都按预期工作。"
                }

                div { class: "guide-overview",
                    span { class: "stat-pill stat-pill-active", "已安装并自动登录" }
                    span { class: "stat-pill", "存储：{storage_label}" }
                    span { class: if mail_enabled { "stat-pill stat-pill-active" } else { "stat-pill stat-pill-warning" },
                        if mail_enabled { "邮件已启用" } else { "邮件未启用" }
                    }
                    span { class: if favicon_configured { "stat-pill stat-pill-active" } else { "stat-pill" },
                        if favicon_configured { "图标已配置" } else { "图标待补充" }
                    }
                }

                div { class: "guide-task-grid",
                    article { class: "guide-task-card" ,
                        p { class: "guide-step", "Step 1" }
                        h3 { "检查系统设置" }
                        p { "确认站点名称、邮件验证地址、SMTP 和存储配置是否符合当前部署环境。" }
                        button {
                            class: "btn btn-primary",
                            onclick: move |event| on_go_settings.call(event),
                            "打开基础设置"
                        }
                    }
                    article { class: "guide-task-card" ,
                        p { class: "guide-step", "Step 2" }
                        h3 { "检查存储配置" }
                        p { "直接进入存储设置分区，确认本地目录或对象存储参数与当前部署环境一致。" }
                        button {
                            class: "btn",
                            onclick: move |event| on_go_storage.call(event),
                            "打开存储设置"
                        }
                    }
                    article { class: "guide-task-card" ,
                        p { class: "guide-step", "Step 3" }
                        h3 { "验证上传链路" }
                        p { "去上传中心上传一张测试图片，再检查历史图库、回收站和访问链接是否正常。" }
                        button {
                            class: "btn",
                            onclick: move |event| on_go_upload.call(event),
                            "去上传中心"
                        }
                    }
                    article { class: "guide-task-card" ,
                        p { class: "guide-step", "Step 4" }
                        h3 { "核对审计记录" }
                        p { "直接进入审计日志，确认安装完成、设置保存和图片上传都留下了清晰记录。" }
                        button {
                            class: "btn",
                            onclick: move |event| on_go_audit.call(event),
                            "打开审计日志"
                        }
                    }
                }

                div { class: "confirm-actions guide-actions",
                    button {
                        class: "btn btn-ghost",
                        onclick: move |_| on_close.call(()),
                        "稍后处理"
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn first_run_guide_storage_key(email: &str) -> String {
    format!(
        "vansour-image:first-run-guide:v1:{}",
        email.trim().to_lowercase()
    )
}

#[cfg(target_arch = "wasm32")]
fn is_first_run_guide_dismissed(email: &str) -> bool {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(&first_run_guide_storage_key(email)).ok())
        .flatten()
        .as_deref()
        == Some("1")
}

#[cfg(not(target_arch = "wasm32"))]
fn is_first_run_guide_dismissed(_email: &str) -> bool {
    false
}

#[cfg(target_arch = "wasm32")]
fn dismiss_first_run_guide(email: &str) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item(&first_run_guide_storage_key(email), "1");
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn dismiss_first_run_guide(_email: &str) {}

#[cfg(target_arch = "wasm32")]
fn reset_first_run_guide_dismissal(email: &str) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.remove_item(&first_run_guide_storage_key(email));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn reset_first_run_guide_dismissal(_email: &str) {}
