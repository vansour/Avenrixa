mod controllers;
mod state;
mod view;

use crate::app_context::{use_auth_store, use_settings_service, use_toast_store};
use crate::store::SettingsAnchor;
use crate::types::api::AdminSettingsConfig;
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use controllers::{
    AccountSectionController, AdvancedSectionController, AuditSectionController,
    MaintenanceSectionController, SecuritySectionController, SystemSectionController,
    UsersSectionController,
};
use view::{
    ADMIN_SETTINGS_SECTIONS, SettingsSection, USER_SETTINGS_SECTIONS, render_settings_fields,
};

pub use state::SettingsFormState;
pub use view::{render_general_fields, render_storage_fields};

const SETTINGS_LOAD_RETRY_DELAYS_MS: [u32; 3] = [0, 500, 1500];

#[component]
pub fn SettingsPage(
    is_admin: bool,
    #[props(default)] requested_section: Option<SettingsAnchor>,
    #[props(default)] on_site_name_updated: EventHandler<String>,
) -> Element {
    let settings_service = use_settings_service();
    let auth_store = use_auth_store();
    let toast_store = use_toast_store();

    let mut is_loading = use_signal(|| true);
    let mut is_saving = use_signal(|| false);
    let mut error_message = use_signal(String::new);
    let mut loaded_config = use_signal(|| None::<AdminSettingsConfig>);
    let mut reload_tick = use_signal(|| 0_u64);
    let mut active_section = use_signal(move || {
        if is_admin {
            SettingsSection::General
        } else {
            SettingsSection::Account
        }
    });
    let mut applied_requested_section = use_signal(|| None::<SettingsAnchor>);

    let site_name = use_signal(String::new);
    let storage_backend = use_signal(|| "local".to_string());
    let local_storage_path = use_signal(String::new);
    let mail_enabled = use_signal(|| false);
    let mail_smtp_host = use_signal(String::new);
    let mail_smtp_port = use_signal(|| "587".to_string());
    let mail_smtp_user = use_signal(String::new);
    let mail_smtp_password = use_signal(String::new);
    let mail_smtp_password_set = use_signal(|| false);
    let mail_from_email = use_signal(String::new);
    let mail_from_name = use_signal(String::new);
    let mail_link_base_url = use_signal(String::new);
    let s3_endpoint = use_signal(String::new);
    let s3_region = use_signal(String::new);
    let s3_bucket = use_signal(String::new);
    let s3_prefix = use_signal(String::new);
    let s3_access_key = use_signal(String::new);
    let s3_secret_key = use_signal(String::new);
    let s3_secret_key_set = use_signal(|| false);
    let s3_force_path_style = use_signal(|| true);

    let form = SettingsFormState {
        site_name,
        storage_backend,
        local_storage_path,
        mail_enabled,
        mail_smtp_host,
        mail_smtp_port,
        mail_smtp_user,
        mail_smtp_password,
        mail_smtp_password_set,
        mail_from_email,
        mail_from_name,
        mail_link_base_url,
        s3_endpoint,
        s3_region,
        s3_bucket,
        s3_prefix,
        s3_access_key,
        s3_secret_key,
        s3_secret_key_set,
        s3_force_path_style,
    };

    use_effect(move || {
        if !is_admin || requested_section == applied_requested_section() {
            return;
        }

        if let Some(anchor) = requested_section {
            active_section.set(settings_section_from_anchor(anchor));
        }

        applied_requested_section.set(requested_section);
    });

    let _load_settings = use_resource({
        let settings_service = settings_service.clone();
        let toast_store = toast_store.clone();
        let auth_store = auth_store.clone();
        move || {
            let _ = reload_tick();
            let settings_service = settings_service.clone();
            let toast_store = toast_store.clone();
            let auth_store = auth_store.clone();
            let mut form = form;
            async move {
                if !is_admin {
                    is_loading.set(false);
                    error_message.set(String::new());
                    return;
                }

                is_loading.set(true);
                error_message.set(String::new());
                let mut last_error = None;

                for delay_ms in SETTINGS_LOAD_RETRY_DELAYS_MS {
                    if delay_ms > 0 {
                        TimeoutFuture::new(delay_ms).await;
                    }

                    match settings_service.get_admin_settings_config().await {
                        Ok(config) => {
                            loaded_config.set(Some(config.clone()));
                            form.apply_loaded_config(config);
                            is_loading.set(false);
                            return;
                        }
                        Err(err) if err.should_redirect_login() => {
                            last_error = Some(err);
                            break;
                        }
                        Err(err) => last_error = Some(err),
                    }
                }

                if let Some(err) = last_error {
                    if err.should_redirect_login() {
                        auth_store.logout();
                    }
                    let message = format!("加载设置失败: {}", err);
                    error_message.set(message.clone());
                    toast_store.show_error(message);
                }

                is_loading.set(false);
            }
        }
    });

    let settings_service_for_save = settings_service.clone();
    let toast_store_for_save = toast_store.clone();
    let on_site_name_updated_for_save = on_site_name_updated.clone();
    let handle_save = move |_| {
        if is_saving() {
            return;
        }

        if let Err(message) = form.validate() {
            error_message.set(message.clone());
            toast_store_for_save.show_error(message);
            return;
        }

        let req = form.build_update_request();
        let settings_service = settings_service_for_save.clone();
        let toast_store = toast_store_for_save.clone();
        let mut form = form;
        let on_site_name_updated = on_site_name_updated_for_save.clone();
        spawn(async move {
            is_saving.set(true);
            error_message.set(String::new());

            match settings_service.update_admin_settings_config(req).await {
                Ok(config) => {
                    loaded_config.set(Some(config.clone()));
                    form.apply_loaded_config(config.clone());
                    on_site_name_updated.call(config.site_name.clone());
                    toast_store.show_success("设置已保存".to_string());
                    if config.restart_required {
                        toast_store.show_info("部分设置需重启服务后生效".to_string());
                    }
                }
                Err(err) => {
                    let message = format!("保存设置失败: {}", err);
                    error_message.set(message.clone());
                    toast_store.show_error(message);
                }
            }

            is_saving.set(false);
        });
    };

    let handle_refresh = move |_| {
        if is_loading() {
            return;
        }
        reload_tick.set(reload_tick().wrapping_add(1));
    };

    let settings_sections: &[SettingsSection] = if is_admin {
        &ADMIN_SETTINGS_SECTIONS
    } else {
        &USER_SETTINGS_SECTIONS
    };
    let current_section = active_section();
    let is_form_disabled = is_loading() || is_saving();
    let has_restart_notice = loaded_config()
        .as_ref()
        .is_some_and(|config| config.restart_required);
    let changed_fields = loaded_config()
        .as_ref()
        .map(|config| count_config_changes(form, config))
        .unwrap_or(0);
    let has_unsaved_changes = changed_fields > 0;
    let storage_summary = current_storage_summary(form);
    let mail_summary = current_mail_summary(form);
    let hero_subtitle = if is_admin {
        current_section.description()
    } else {
        "管理当前账户信息和密码安全。"
    };

    rsx! {
        div { class: "dashboard-page settings-page",
            section { class: "page-hero settings-hero settings-hero-rich",
                div { class: "settings-hero-main settings-hero-main-stack",
                    div {
                        p { class: "settings-eyebrow",
                            if is_admin { "Admin Console" } else { "Account Console" }
                        }
                        h1 { if is_admin { "系统设置" } else { "账户设置" } }
                        p { class: "settings-hero-copy", "{hero_subtitle}" }
                    }
                    div { class: "settings-pill-row",
                        span { class: "stat-pill stat-pill-active",
                            if is_admin { "管理员视图" } else { "用户视图" }
                        }
                        if is_admin {
                            span { class: "stat-pill", "存储：{storage_summary}" }
                            span { class: "stat-pill", "邮件：{mail_summary}" }
                            span { class: if has_unsaved_changes { "stat-pill stat-pill-warning" } else { "stat-pill" },
                                if has_unsaved_changes {
                                    "待保存：{changed_fields} 项"
                                } else {
                                    "当前无未保存修改"
                                }
                            }
                            span { class: if has_restart_notice { "stat-pill stat-pill-warning" } else { "stat-pill" },
                                if has_restart_notice {
                                    "部分配置需重启"
                                } else {
                                    "无需重启"
                                }
                            }
                        }
                    }
                }
            }

            div { class: "settings-workspace",
                aside { class: "settings-sidebar",
                    div { class: "settings-sidebar-card settings-sidebar-intro",
                        p { class: "settings-eyebrow", "当前分区" }
                        h2 { "{current_section.label()}" }
                        p { class: "settings-sidebar-copy", "{current_section.description()}" }
                        if is_admin {
                            div { class: "settings-sidebar-meta",
                                span { class: "stat-pill", "存储：{storage_summary}" }
                                span { class: "stat-pill", "邮件：{mail_summary}" }
                            }
                        }
                    }

                    nav { class: "settings-sidebar-card settings-nav",
                        for section in settings_sections.iter().copied() {
                            button {
                                r#type: "button",
                                class: if section == current_section {
                                    "settings-nav-item is-active"
                                } else {
                                    "settings-nav-item"
                                },
                                onclick: move |_| active_section.set(section),
                                div { class: "settings-nav-copy",
                                    strong { "{section.label()}" }
                                    small { "{section.description()}" }
                                }
                            }
                        }
                    }
                }

                div { class: "settings-panel-column",
                    section { class: "settings-card settings-panel-card",
                        div { class: "settings-panel-head",
                            div {
                                h2 { class: "settings-panel-title", "{current_section.title()}" }
                                p { class: "settings-panel-copy", "{current_section.description()}" }
                            }
                            if current_section.uses_global_settings_actions() {
                                div { class: "settings-panel-badges",
                                    span { class: if has_unsaved_changes { "stat-pill stat-pill-warning" } else { "stat-pill" },
                                        if has_unsaved_changes {
                                            "未保存 {changed_fields} 项"
                                        } else {
                                            "草稿已同步"
                                        }
                                    }
                                    if has_restart_notice {
                                        span { class: "stat-pill stat-pill-warning", "保存后可能需要重启" }
                                    }
                                }
                            }
                        }

                        if !error_message().is_empty() && current_section.uses_global_settings_actions() {
                            div { class: "error-banner", "{error_message()}" }
                        }

                        if current_section.uses_global_settings_actions() && has_unsaved_changes {
                            div { class: "settings-banner settings-banner-neutral",
                                "当前草稿与已生效配置存在差异。保存前请再次确认关键路径、SMTP 参数或对象存储信息。"
                            }
                        }

                        if current_section.uses_global_settings_actions() && has_restart_notice {
                            div { class: "settings-banner settings-banner-neutral",
                                "当前已生效配置里包含需要重启后端才能完全切换的项目，通常是存储后端、存储路径或 S3 连接信息。"
                            }
                        }

                        {
                            match current_section {
                                SettingsSection::Account => rsx! { AccountSectionController {} },
                                SettingsSection::Security => rsx! { SecuritySectionController {} },
                                SettingsSection::System => rsx! { SystemSectionController {} },
                                SettingsSection::Maintenance => rsx! { MaintenanceSectionController {} },
                                SettingsSection::Users => rsx! { UsersSectionController {} },
                                SettingsSection::Audit => rsx! { AuditSectionController {} },
                                SettingsSection::Advanced => rsx! {
                                    AdvancedSectionController { on_site_name_updated }
                                },
                                _ => render_settings_fields(form, is_form_disabled, current_section),
                            }
                        }

                        if current_section.uses_global_settings_actions() {
                            div { class: "settings-actions",
                                button {
                                    class: "btn",
                                    onclick: handle_refresh,
                                    disabled: is_form_disabled,
                                    if is_loading() { "加载中..." } else { "刷新" }
                                }
                                button {
                                    class: "btn btn-primary",
                                    onclick: handle_save,
                                    disabled: is_form_disabled,
                                    if is_saving() { "保存中..." } else { "保存设置" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn settings_section_from_anchor(anchor: SettingsAnchor) -> SettingsSection {
    match anchor {
        SettingsAnchor::Account => SettingsSection::Account,
        SettingsAnchor::General => SettingsSection::General,
        SettingsAnchor::Storage => SettingsSection::Storage,
        SettingsAnchor::Security => SettingsSection::Security,
        SettingsAnchor::System => SettingsSection::System,
        SettingsAnchor::Maintenance => SettingsSection::Maintenance,
        SettingsAnchor::Users => SettingsSection::Users,
        SettingsAnchor::Audit => SettingsSection::Audit,
        SettingsAnchor::Advanced => SettingsSection::Advanced,
    }
}

fn count_config_changes(form: SettingsFormState, config: &AdminSettingsConfig) -> usize {
    let mut changes = 0;

    if (form.site_name)().trim() != config.site_name.trim() {
        changes += 1;
    }
    if (form.storage_backend)().trim() != config.storage_backend.trim() {
        changes += 1;
    }
    if (form.local_storage_path)().trim() != config.local_storage_path.trim() {
        changes += 1;
    }
    if (form.mail_enabled)() != config.mail_enabled {
        changes += 1;
    }
    if (form.mail_smtp_host)().trim() != config.mail_smtp_host.trim() {
        changes += 1;
    }
    if (form.mail_smtp_port)().trim() != config.mail_smtp_port.to_string() {
        changes += 1;
    }
    if !trimmed_option_eq((form.mail_smtp_user)(), config.mail_smtp_user.clone()) {
        changes += 1;
    }
    if !(form.mail_smtp_password)().trim().is_empty() {
        changes += 1;
    }
    if (form.mail_from_email)().trim() != config.mail_from_email.trim() {
        changes += 1;
    }
    if (form.mail_from_name)().trim() != config.mail_from_name.trim() {
        changes += 1;
    }
    if (form.mail_link_base_url)().trim() != config.mail_link_base_url.trim() {
        changes += 1;
    }
    if !trimmed_option_eq((form.s3_endpoint)(), config.s3_endpoint.clone()) {
        changes += 1;
    }
    if !trimmed_option_eq((form.s3_region)(), config.s3_region.clone()) {
        changes += 1;
    }
    if !trimmed_option_eq((form.s3_bucket)(), config.s3_bucket.clone()) {
        changes += 1;
    }
    if !trimmed_option_eq((form.s3_prefix)(), config.s3_prefix.clone()) {
        changes += 1;
    }
    if !trimmed_option_eq((form.s3_access_key)(), config.s3_access_key.clone()) {
        changes += 1;
    }
    if !(form.s3_secret_key)().trim().is_empty() {
        changes += 1;
    }
    if (form.s3_force_path_style)() != config.s3_force_path_style {
        changes += 1;
    }

    changes
}

fn trimmed_option_eq(draft: String, current: Option<String>) -> bool {
    let draft = draft.trim();
    let current = current.unwrap_or_default();
    draft == current.trim()
}

fn current_storage_summary(form: SettingsFormState) -> &'static str {
    if form.is_s3_backend() {
        "S3 / MinIO"
    } else {
        "本地目录"
    }
}

fn current_mail_summary(form: SettingsFormState) -> &'static str {
    if (form.mail_enabled)() {
        "已启用"
    } else {
        "未启用"
    }
}
