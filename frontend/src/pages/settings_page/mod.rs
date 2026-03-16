mod controllers;
mod state;
mod view;

use crate::app_context::{use_auth_store, use_settings_service, use_toast_store};
use crate::auth_session::{auth_session_expired_message, handle_auth_session_error};
use crate::store::{AuthStore, SettingsAnchor, ToastStore};
use crate::types::api::{AdminSettingsConfig, StorageBackendKind, TestS3StorageConfigRequest};
use crate::types::errors::AppError;
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use controllers::{
    AccountSectionController, MaintenanceSectionController, SecuritySectionController,
    SystemSectionController, UsersSectionController,
};
use view::{
    ADMIN_SETTINGS_SECTIONS, SettingsSection, USER_SETTINGS_SECTIONS, render_settings_fields,
};

use state::display_mail_smtp_port;
pub use state::infer_s3_provider_preset;
pub use state::{S3ProviderPreset, SettingsFormState, default_mail_link_base_url};
pub use view::{
    render_general_fields, render_general_fields_compact, render_s3_fields,
    render_s3_fields_compact, render_storage_fields, render_storage_fields_compact,
};

const SETTINGS_LOAD_RETRY_DELAYS_MS: [u32; 3] = [0, 500, 1500];

#[derive(Clone, Copy, PartialEq, Eq)]
enum S3TestFeedbackTone {
    Neutral,
    Success,
    Error,
}

pub(super) fn settings_auth_expired_message() -> String {
    auth_session_expired_message()
}

pub(super) fn handle_settings_auth_error(
    auth_store: &AuthStore,
    toast_store: &ToastStore,
    err: &AppError,
) -> bool {
    handle_auth_session_error(auth_store, toast_store, err)
}

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
    let mut is_testing_s3 = use_signal(|| false);
    let mut error_message = use_signal(String::new);
    let mut loaded_config = use_signal(|| None::<AdminSettingsConfig>);
    let mut last_tested_s3_request = use_signal(|| None::<TestS3StorageConfigRequest>);
    let mut s3_test_feedback = use_signal(String::new);
    let mut s3_test_feedback_tone = use_signal(|| S3TestFeedbackTone::Neutral);
    let mut reload_tick = use_signal(|| 0_u64);
    let mut last_loaded_tick = use_signal(|| None::<u64>);
    let mut active_section = use_signal(move || {
        if is_admin {
            SettingsSection::General
        } else {
            SettingsSection::Account
        }
    });
    let mut applied_requested_section = use_signal(|| None::<SettingsAnchor>);

    let site_name = use_signal(String::new);
    let storage_backend = use_signal(|| StorageBackendKind::Unknown);
    let local_storage_path = use_signal(String::new);
    let mail_enabled = use_signal(|| false);
    let mail_smtp_host = use_signal(String::new);
    let mail_smtp_port = use_signal(String::new);
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
    let s3_provider_preset = use_signal(|| S3ProviderPreset::Other);
    let s3_provider_drafts = use_signal(std::collections::BTreeMap::new);

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
        s3_provider_preset,
        s3_provider_drafts,
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

    use_effect({
        let settings_service = settings_service.clone();
        let toast_store = toast_store.clone();
        let auth_store = auth_store.clone();
        move || {
            let current_tick = reload_tick();
            if last_loaded_tick() == Some(current_tick) {
                return;
            }
            last_loaded_tick.set(Some(current_tick));
            let settings_service = settings_service.clone();
            let toast_store = toast_store.clone();
            let auth_store = auth_store.clone();
            let mut form = form;
            spawn(async move {
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
                            last_tested_s3_request.set(None);
                            s3_test_feedback.set(String::new());
                            s3_test_feedback_tone.set(S3TestFeedbackTone::Neutral);
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
                    if handle_settings_auth_error(&auth_store, &toast_store, &err) {
                        error_message.set(settings_auth_expired_message());
                    } else {
                        let message = format!("加载设置失败: {}", err);
                        error_message.set(message.clone());
                        toast_store.show_error(message);
                    }
                }

                is_loading.set(false);
            });
        }
    });

    let settings_service_for_save = settings_service.clone();
    let settings_service_for_test = settings_service.clone();
    let auth_store_for_save = auth_store.clone();
    let toast_store_for_save = toast_store.clone();
    let on_site_name_updated_for_save = on_site_name_updated;
    let handle_save = move |_| {
        if is_saving() {
            return;
        }

        let requires_s3_test = loaded_config()
            .as_ref()
            .is_some_and(|config| requires_s3_test_confirmation(form, config));
        if requires_s3_test && !is_current_s3_request_confirmed(form, last_tested_s3_request()) {
            let message = "请先完成 S3 连通性测试，再保存当前配置".to_string();
            error_message.set(message.clone());
            toast_store_for_save.show_error(message);
            return;
        }

        if let Err(message) = form.validate() {
            error_message.set(message.clone());
            toast_store_for_save.show_error(message);
            return;
        }

        let req = form.build_update_request();
        let settings_service = settings_service_for_save.clone();
        let auth_store = auth_store_for_save.clone();
        let toast_store = toast_store_for_save.clone();
        let mut form = form;
        let on_site_name_updated = on_site_name_updated_for_save;
        spawn(async move {
            is_saving.set(true);
            error_message.set(String::new());

            match settings_service.update_admin_settings_config(req).await {
                Ok(config) => {
                    loaded_config.set(Some(config.clone()));
                    form.apply_loaded_config(config.clone());
                    last_tested_s3_request.set(None);
                    s3_test_feedback.set(String::new());
                    s3_test_feedback_tone.set(S3TestFeedbackTone::Neutral);
                    on_site_name_updated.call(config.site_name.clone());
                    toast_store.show_success("设置已保存".to_string());
                }
                Err(err) => {
                    if handle_settings_auth_error(&auth_store, &toast_store, &err) {
                        error_message.set(settings_auth_expired_message());
                    } else {
                        let message = format!("保存设置失败: {}", err);
                        error_message.set(message.clone());
                        toast_store.show_error(message);
                    }
                }
            }

            is_saving.set(false);
        });
    };

    let handle_test_s3 = move |_| {
        if is_testing_s3() || is_loading() || is_saving() {
            return;
        }

        if let Err(message) = form.validate_s3_for_test() {
            error_message.set(message.clone());
            s3_test_feedback.set(message.clone());
            s3_test_feedback_tone.set(S3TestFeedbackTone::Error);
            toast_store.show_error(message);
            return;
        }

        let req = form.build_s3_test_request();
        let settings_service = settings_service_for_test.clone();
        let toast_store = toast_store.clone();
        let auth_store = auth_store.clone();
        spawn(async move {
            is_testing_s3.set(true);
            error_message.set(String::new());
            s3_test_feedback.set(String::new());
            s3_test_feedback_tone.set(S3TestFeedbackTone::Neutral);

            match settings_service.test_s3_storage_config(req.clone()).await {
                Ok(response) => {
                    last_tested_s3_request.set(Some(req));
                    s3_test_feedback.set(response.message.clone());
                    s3_test_feedback_tone.set(S3TestFeedbackTone::Success);
                    toast_store.show_success(response.message);
                }
                Err(err) => {
                    let message = format!("S3 测试失败: {}", err);
                    last_tested_s3_request.set(None);
                    s3_test_feedback.set(message.clone());
                    s3_test_feedback_tone.set(S3TestFeedbackTone::Error);
                    if handle_settings_auth_error(&auth_store, &toast_store, &err) {
                        error_message.set(settings_auth_expired_message());
                    } else {
                        error_message.set(message.clone());
                        toast_store.show_error(message);
                    }
                }
            }

            is_testing_s3.set(false);
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
    let requires_s3_test = loaded_config()
        .as_ref()
        .is_some_and(|config| requires_s3_test_confirmation(form, config));
    let s3_test_confirmed =
        !requires_s3_test || is_current_s3_request_confirmed(form, last_tested_s3_request());
    let is_form_disabled = is_loading() || is_saving() || is_testing_s3();
    let save_disabled = is_form_disabled || (requires_s3_test && !s3_test_confirmed);
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
    let page_title = if is_admin {
        "系统设置"
    } else {
        "账户设置"
    };
    let page_eyebrow = if is_admin {
        "Admin Console"
    } else {
        "Account Console"
    };
    rsx! {
        div { class: "dashboard-page settings-page",
            section { class: "settings-card settings-header",
                div { class: "settings-header-main",
                    div {
                        p { class: "settings-eyebrow", "{page_eyebrow}" }
                        h1 { "{page_title}" }
                    }
                    div { class: "settings-pill-row",
                        span { class: "stat-pill stat-pill-active", "{current_section.label()}" }
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
                    nav { class: "settings-card settings-nav",
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

                        {
                            match current_section {
                                SettingsSection::Account => rsx! { AccountSectionController {} },
                                SettingsSection::Security => rsx! { SecuritySectionController {} },
                                SettingsSection::System => rsx! { SystemSectionController {} },
                                SettingsSection::Maintenance => rsx! { MaintenanceSectionController {} },
                                SettingsSection::Users => rsx! { UsersSectionController {} },
                                _ => render_settings_fields(form, is_form_disabled, current_section),
                            }
                        }

                        if current_section == SettingsSection::Storage && form.is_s3_backend() {
                            div { class: "settings-stack",
                                div { class: "settings-actions",
                                    button {
                                        class: "btn btn-primary",
                                        onclick: handle_test_s3,
                                        disabled: is_form_disabled,
                                        if is_testing_s3() {
                                            "测试中..."
                                        } else {
                                            "测试 S3 连通性"
                                        }
                                    }
                                }

                                if !s3_test_feedback().is_empty() {
                                    div {
                                        class: match s3_test_feedback_tone() {
                                            S3TestFeedbackTone::Success => "settings-banner settings-banner-success",
                                            S3TestFeedbackTone::Error => "error-banner",
                                            S3TestFeedbackTone::Neutral => "settings-banner settings-banner-warning",
                                        },
                                        "{s3_test_feedback()}"
                                    }
                                } else if requires_s3_test {
                                    div { class: "settings-banner settings-banner-warning",
                                        "S3 配置已变更，请先完成连通性测试后再保存。"
                                    }
                                }
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
                                    disabled: save_disabled,
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
    }
}

fn count_config_changes(form: SettingsFormState, config: &AdminSettingsConfig) -> usize {
    let mut changes = 0;

    if (form.site_name)().trim() != config.site_name.trim() {
        changes += 1;
    }
    if (form.storage_backend)() != config.storage_backend {
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
    if (form.mail_smtp_port)().trim() != display_mail_smtp_port(config.mail_smtp_port) {
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

fn requires_s3_test_confirmation(form: SettingsFormState, config: &AdminSettingsConfig) -> bool {
    if !form.is_s3_backend() {
        return false;
    }

    if config.storage_backend != StorageBackendKind::S3 {
        return true;
    }

    has_s3_config_changes(form, config)
}

fn has_s3_config_changes(form: SettingsFormState, config: &AdminSettingsConfig) -> bool {
    !trimmed_option_eq((form.s3_endpoint)(), config.s3_endpoint.clone())
        || !trimmed_option_eq((form.s3_region)(), config.s3_region.clone())
        || !trimmed_option_eq((form.s3_bucket)(), config.s3_bucket.clone())
        || !trimmed_option_eq((form.s3_prefix)(), config.s3_prefix.clone())
        || !trimmed_option_eq((form.s3_access_key)(), config.s3_access_key.clone())
        || !(form.s3_secret_key)().trim().is_empty()
        || (form.s3_force_path_style)() != config.s3_force_path_style
}

fn is_current_s3_request_confirmed(
    form: SettingsFormState,
    last_tested_request: Option<TestS3StorageConfigRequest>,
) -> bool {
    last_tested_request.is_some_and(|tested| tested == form.build_s3_test_request())
}

fn trimmed_option_eq(draft: String, current: Option<String>) -> bool {
    let draft = draft.trim();
    let current = current.unwrap_or_default();
    draft == current.trim()
}

fn current_storage_summary(form: SettingsFormState) -> String {
    match (form.storage_backend)() {
        StorageBackendKind::Unknown => "未选择".to_string(),
        StorageBackendKind::Local => {
            let path = (form.local_storage_path)().trim().to_string();
            if path.is_empty() {
                "本地目录".to_string()
            } else {
                format!("本地目录 · {path}")
            }
        }
        StorageBackendKind::S3 => {
            let bucket = (form.s3_bucket)().trim().to_string();
            let prefix = (form.s3_prefix)().trim().trim_matches('/').to_string();
            let preset = (form.s3_provider_preset)();
            let provider_label = match preset {
                S3ProviderPreset::Other => "对象存储",
                _ => preset.label(),
            };

            if bucket.is_empty() {
                provider_label.to_string()
            } else if prefix.is_empty() {
                format!("{provider_label} · {bucket}")
            } else {
                format!("{provider_label} · {bucket}/{prefix}")
            }
        }
    }
}

fn current_mail_summary(form: SettingsFormState) -> &'static str {
    if (form.mail_enabled)() {
        "已启用"
    } else {
        "未启用"
    }
}
