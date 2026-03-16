use crate::app_context::{use_install_service, use_toast_store};
use crate::components::Modal;
use crate::pages::settings_page::{
    SettingsFormState, default_mail_link_base_url, infer_s3_provider_preset,
    render_general_fields_compact, render_s3_fields_compact,
};
use crate::services::InstallService;
use crate::types::api::{
    AdminSettingsConfig, BootstrapStatusResponse, InstallBootstrapRequest,
    InstallBootstrapResponse, StorageBackendKind, StorageDirectoryEntry,
    TestS3StorageConfigRequest,
};
use base64::Engine;
use dioxus::html::FileData;
use dioxus::prelude::*;

const MIN_ADMIN_PASSWORD_LENGTH: usize = 12;
const DEFAULT_INSTALL_SITE_NAME: &str = "Vansour Image";
const DEFAULT_INSTALL_STORAGE_BROWSER_PATH: &str = "/";
const INSTALL_WIZARD_STEPS: [InstallWizardStep; 4] = [
    InstallWizardStep::Admin,
    InstallWizardStep::General,
    InstallWizardStep::Storage,
    InstallWizardStep::Review,
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum S3TestFeedbackTone {
    Neutral,
    Success,
    Error,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InstallWizardStep {
    Admin,
    General,
    Storage,
    Review,
}

impl InstallWizardStep {
    fn label(self) -> &'static str {
        match self {
            Self::Admin => "创建管理员账号",
            Self::General => "配置站点信息",
            Self::Storage => "确认存储方案",
            Self::Review => "检查并初始化",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Admin => "第 1 步：创建管理员账号",
            Self::General => "第 2 步：配置站点信息",
            Self::Storage => "第 3 步：确认存储方案",
            Self::Review => "第 4 步：检查并初始化",
        }
    }
}

fn initial_local_storage_path(config: &AdminSettingsConfig) -> String {
    config.local_storage_path.trim().to_string()
}

fn initial_site_name(config: &AdminSettingsConfig) -> String {
    let site_name = config.site_name.trim();
    if site_name == DEFAULT_INSTALL_SITE_NAME {
        String::new()
    } else {
        site_name.to_string()
    }
}

fn initial_storage_backend(config: &AdminSettingsConfig) -> StorageBackendKind {
    let has_local_path = !config.local_storage_path.trim().is_empty();
    let has_s3_config = config
        .s3_endpoint
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty())
        || config
            .s3_region
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
        || config
            .s3_bucket
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
        || config
            .s3_access_key
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
        || config.s3_secret_key_set;

    if !has_local_path && !has_s3_config {
        StorageBackendKind::Unknown
    } else {
        config.storage_backend
    }
}

fn initial_mail_smtp_port(config: &AdminSettingsConfig) -> String {
    if config.mail_smtp_port == 0 {
        String::new()
    } else {
        config.mail_smtp_port.to_string()
    }
}

#[component]
pub fn InstallWizardPage(
    bootstrap_status: BootstrapStatusResponse,
    initial_config: AdminSettingsConfig,
    #[props(default)] on_installed: EventHandler<InstallBootstrapResponse>,
) -> Element {
    let install_service = use_install_service();
    let toast_store = use_toast_store();

    let site_name = use_signal({
        let initial = initial_site_name(&initial_config);
        move || initial.clone()
    });
    let storage_backend = use_signal({
        let initial = initial_storage_backend(&initial_config);
        move || initial
    });
    let local_storage_path = use_signal({
        let initial = initial_local_storage_path(&initial_config);
        move || initial.clone()
    });
    let mail_enabled = use_signal({
        let initial = initial_config.mail_enabled;
        move || initial
    });
    let mail_smtp_host = use_signal({
        let initial = initial_config.mail_smtp_host.clone();
        move || initial.clone()
    });
    let mail_smtp_port = use_signal({
        let initial = initial_mail_smtp_port(&initial_config);
        move || initial.clone()
    });
    let mail_smtp_user = use_signal({
        let initial = initial_config.mail_smtp_user.clone().unwrap_or_default();
        move || initial.clone()
    });
    let mail_smtp_password = use_signal(String::new);
    let mail_smtp_password_set = use_signal({
        let initial = initial_config.mail_smtp_password_set;
        move || initial
    });
    let mail_from_email = use_signal({
        let initial = initial_config.mail_from_email.clone();
        move || initial.clone()
    });
    let mail_from_name = use_signal({
        let initial = initial_config.mail_from_name.clone();
        move || initial.clone()
    });
    let mail_link_base_url = use_signal({
        let initial = default_mail_link_base_url(&initial_config.mail_link_base_url);
        move || initial.clone()
    });
    let s3_endpoint = use_signal({
        let initial = initial_config.s3_endpoint.clone().unwrap_or_default();
        move || initial.clone()
    });
    let s3_region = use_signal({
        let initial = initial_config.s3_region.clone().unwrap_or_default();
        move || initial.clone()
    });
    let s3_bucket = use_signal({
        let initial = initial_config.s3_bucket.clone().unwrap_or_default();
        move || initial.clone()
    });
    let s3_prefix = use_signal({
        let initial = initial_config.s3_prefix.clone().unwrap_or_default();
        move || initial.clone()
    });
    let s3_access_key = use_signal({
        let initial = initial_config.s3_access_key.clone().unwrap_or_default();
        move || initial.clone()
    });
    let s3_secret_key = use_signal(String::new);
    let s3_secret_key_set = use_signal({
        let initial = initial_config.s3_secret_key_set;
        move || initial
    });
    let s3_force_path_style = use_signal({
        let initial = initial_config.s3_force_path_style;
        move || initial
    });
    let s3_provider_preset = use_signal({
        let initial = infer_s3_provider_preset(
            initial_config.s3_endpoint.as_deref(),
            initial_config.s3_region.as_deref(),
            initial_config.s3_force_path_style,
        );
        move || initial
    });
    let s3_provider_drafts = use_signal(std::collections::BTreeMap::new);

    let mut form = SettingsFormState {
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

    let mut admin_email = use_signal(String::new);
    let mut admin_password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut show_admin_password = use_signal(|| false);
    let mut show_confirm_password = use_signal(|| false);
    let mut selected_favicon = use_signal(|| None::<FileData>);
    let mut current_step = use_signal(|| InstallWizardStep::Admin);
    let mut is_installing = use_signal(|| false);
    let mut error_message = use_signal(String::new);
    let mut success_message = use_signal(String::new);
    let mut is_testing_s3 = use_signal(|| false);
    let mut last_tested_s3_request = use_signal(|| None::<TestS3StorageConfigRequest>);
    let mut s3_test_feedback = use_signal(String::new);
    let mut s3_test_feedback_tone = use_signal(|| S3TestFeedbackTone::Neutral);
    let mut storage_browser_open = use_signal(|| false);
    let storage_browser_loading = use_signal(|| false);
    let mut storage_browser_error = use_signal(String::new);
    let storage_browser_current_path =
        use_signal(|| DEFAULT_INSTALL_STORAGE_BROWSER_PATH.to_string());
    let storage_browser_parent_path = use_signal(|| None::<String>);
    let storage_browser_directories = use_signal(Vec::<StorageDirectoryEntry>::new);

    let handle_pick_favicon = move |event: Event<FormData>| {
        let mut files = event.files().into_iter();
        match files.next() {
            Some(file) => selected_favicon.set(Some(file)),
            None => selected_favicon.set(None),
        }
    };

    let install_service_for_submit = install_service.clone();
    let install_service_for_test = install_service.clone();
    let toast_store_for_install = toast_store.clone();
    let toast_store_for_test = toast_store.clone();
    let mut handle_install = move || {
        if is_installing() || is_testing_s3() {
            return;
        }

        let email = admin_email().trim().to_string();
        let password = admin_password();
        let confirm = confirm_password();
        if let Some(message) =
            install_admin_submit_error(email.as_str(), password.as_str(), confirm.as_str())
        {
            error_message.set(message.clone());
            toast_store_for_install.show_error(message);
            return;
        }
        if let Err(message) = form.validate() {
            error_message.set(message.clone());
            toast_store_for_install.show_error(message);
            return;
        }
        if form.is_s3_backend()
            && !is_current_install_s3_request_confirmed(form, last_tested_s3_request())
        {
            let message = "请先完成 S3 连通性测试，再继续安装".to_string();
            error_message.set(message.clone());
            toast_store_for_install.show_error(message);
            return;
        }

        let install_service = install_service_for_submit.clone();
        let toast_store = toast_store_for_install.clone();
        let req_config = form.build_update_request();
        let favicon_file = selected_favicon();

        spawn(async move {
            is_installing.set(true);
            error_message.set(String::new());
            success_message.set(String::new());

            let favicon_data_url = match favicon_file {
                Some(file) => match favicon_file_to_data_url(file).await {
                    Ok(data_url) => Some(data_url),
                    Err(message) => {
                        error_message.set(message.clone());
                        toast_store.show_error(message);
                        is_installing.set(false);
                        return;
                    }
                },
                None => None,
            };

            let request = InstallBootstrapRequest {
                admin_email: email,
                admin_password: password,
                favicon_data_url,
                config: req_config,
            };

            match install_service.bootstrap_installation(request).await {
                Ok(response) => {
                    let message = "安装完成，已自动登录管理员账户".to_string();
                    success_message.set(message.clone());
                    toast_store.show_success(message);
                    on_installed.call(response);
                }
                Err(err) => {
                    let message = format!("安装失败: {}", err);
                    error_message.set(message.clone());
                    toast_store.show_error(message);
                }
            }

            is_installing.set(false);
        });
    };

    let handle_test_s3 = move |_| {
        if is_installing() || is_testing_s3() {
            return;
        }

        if let Err(message) = form.validate_s3_for_test() {
            error_message.set(message.clone());
            s3_test_feedback.set(message.clone());
            s3_test_feedback_tone.set(S3TestFeedbackTone::Error);
            toast_store_for_test.show_error(message);
            return;
        }

        let req = form.build_s3_test_request();
        let install_service = install_service_for_test.clone();
        let toast_store = toast_store_for_test.clone();
        spawn(async move {
            is_testing_s3.set(true);
            error_message.set(String::new());
            s3_test_feedback.set(String::new());
            s3_test_feedback_tone.set(S3TestFeedbackTone::Neutral);

            match install_service.test_s3_storage_config(req.clone()).await {
                Ok(response) => {
                    last_tested_s3_request.set(Some(req));
                    s3_test_feedback.set(response.message.clone());
                    s3_test_feedback_tone.set(S3TestFeedbackTone::Success);
                    toast_store.show_success(response.message);
                }
                Err(err) => {
                    let message = format!("S3 测试失败: {}", err);
                    last_tested_s3_request.set(None);
                    error_message.set(message.clone());
                    s3_test_feedback.set(message.clone());
                    s3_test_feedback_tone.set(S3TestFeedbackTone::Error);
                    toast_store.show_error(message);
                }
            }

            is_testing_s3.set(false);
        });
    };

    let handle_primary_action = move |_| {
        let step = current_step();
        if step == InstallWizardStep::Review {
            handle_install();
            return;
        }

        let step_index = install_step_index(step);
        if let Some(next) = INSTALL_WIZARD_STEPS.get(step_index + 1).copied() {
            current_step.set(next);
        }
    };

    let admin_email_value = admin_email();
    let admin_password_value = admin_password();
    let confirm_password_value = confirm_password();
    let admin_email_error = install_admin_email_error(admin_email_value.as_str());
    let admin_password_error = install_admin_password_error(admin_password_value.as_str());
    let confirm_password_error = install_admin_confirm_password_error(
        admin_password_value.as_str(),
        confirm_password_value.as_str(),
    );
    let selected_favicon_name = selected_favicon()
        .as_ref()
        .map(|file| file.name())
        .unwrap_or_default();
    let has_custom_favicon = !selected_favicon_name.is_empty();
    let favicon_summary = if has_custom_favicon {
        selected_favicon_name.clone()
    } else {
        "后续可再上传".to_string()
    };
    let admin_ready = install_admin_ready(
        admin_email_value.as_str(),
        admin_password_value.as_str(),
        confirm_password_value.as_str(),
    );
    let site_ready = !(form.site_name)().trim().is_empty();
    let mail_ready = install_mail_ready(form);
    let general_ready = site_ready && mail_ready;
    let s3_test_confirmed = !form.is_s3_backend()
        || is_current_install_s3_request_confirmed(form, last_tested_s3_request());
    let storage_ready = install_storage_ready(form, s3_test_confirmed);
    let review_ready = admin_ready && general_ready && storage_ready;
    let storage_summary = match (form.storage_backend)() {
        StorageBackendKind::Local => "本地存储".to_string(),
        StorageBackendKind::S3 => "对象存储（兼容 S3）".to_string(),
        StorageBackendKind::Unknown => "待选择".to_string(),
    };
    let mail_summary = if !(form.mail_enabled)() {
        "未启用".to_string()
    } else if mail_ready {
        "已启用".to_string()
    } else {
        "待补全".to_string()
    };
    let environment_source = if bootstrap_status.mode == "runtime" {
        "部署环境预设"
    } else {
        "Bootstrap 配置文件"
    };
    let database_label = bootstrap_status.database_kind.label().to_string();
    let database_status = if bootstrap_status.database_configured {
        "已读取部署配置".to_string()
    } else {
        "未提供配置".to_string()
    };
    let database_connection = bootstrap_status
        .database_url_masked
        .clone()
        .unwrap_or_else(|| "未检测到数据库连接".to_string());
    let database_status_class = if bootstrap_status.database_configured {
        "stat-pill stat-pill-active"
    } else {
        "stat-pill stat-pill-warning"
    };
    let cache_label = if bootstrap_status.cache_configured {
        "Redis 外部缓存".to_string()
    } else {
        "无外部缓存".to_string()
    };
    let cache_status = if bootstrap_status.cache_configured {
        "已读取缓存配置".to_string()
    } else {
        "未启用外部缓存".to_string()
    };
    let cache_connection = bootstrap_status
        .cache_url_masked
        .clone()
        .unwrap_or_else(|| "未配置 REDIS_URL".to_string());
    let cache_status_class = if bootstrap_status.cache_configured {
        "stat-pill stat-pill-active"
    } else {
        "stat-pill"
    };
    let runtime_error = bootstrap_status.runtime_error.clone().unwrap_or_default();
    let site_name_summary = summary_or_pending((form.site_name)());
    let completed_steps_count = [admin_ready, general_ready, storage_ready, review_ready]
        .into_iter()
        .filter(|done| *done)
        .count();

    let current_step_value = current_step();
    let current_step_index = install_step_index(current_step_value);
    let current_step_ready = install_step_complete(
        current_step_value,
        admin_ready,
        general_ready,
        storage_ready,
        review_ready,
    );
    let total_steps = INSTALL_WIZARD_STEPS.len();
    let prev_step = current_step_index
        .checked_sub(1)
        .map(|index| INSTALL_WIZARD_STEPS[index]);
    let next_step = INSTALL_WIZARD_STEPS.get(current_step_index + 1).copied();
    let is_review_step = current_step_value == InstallWizardStep::Review;
    let primary_action_label = if is_review_step {
        if is_installing() {
            "正在安装...".to_string()
        } else {
            "完成安装并创建管理员".to_string()
        }
    } else {
        format!(
            "下一步：{}",
            next_step
                .map(InstallWizardStep::label)
                .unwrap_or("确认安装")
        )
    };
    let open_browser_service = install_service.clone();
    let browse_parent_service = install_service.clone();
    let browser_directories = storage_browser_directories();
    let install_progress_percent = ((current_step_index + 1) as f64 / total_steps as f64) * 100.0;

    rsx! {
        div { class: "dashboard-page settings-page install-page install-page-wizard",
            div { class: "install-wizard-shell",
                section { class: "settings-card settings-header install-header-card",
                    div { class: "settings-header-main install-header-main",
                        div { class: "install-header-copy",
                            p { class: "settings-eyebrow", "安装流程" }
                            h1 { "安装向导" }
                        }
                        div { class: "install-header-status",
                            div { class: "settings-pill-row install-header-pills",
                                span { class: "stat-pill stat-pill-active", "当前进行中 {current_step_index + 1}/{total_steps}" }
                                span { class: "stat-pill", "已完成 {completed_steps_count}/{total_steps}" }
                                span { class: "stat-pill", "环境来源：{environment_source}" }
                            }
                            div { class: "install-progress",
                                div { class: "install-progress-meta",
                                    strong { "{current_step_value.label()}" }
                                    span { "完成度 {install_progress_percent.round() as i32}%" }
                                }
                                div { class: "install-progress-bar",
                                    span {
                                        class: "install-progress-fill",
                                        style: "width: {install_progress_percent}%",
                                    }
                                }
                            }
                        }
                    }
                }

                nav { class: "settings-card install-step-track", aria_label: "安装步骤",
                    {INSTALL_WIZARD_STEPS.iter().copied().enumerate().map(|(index, step)| {
                        let mut current_step = current_step;
                        let state_class = install_step_state_class(
                            step,
                            current_step_value,
                            admin_ready,
                            general_ready,
                            storage_ready,
                            review_ready,
                        );
                        rsx! {
                            button {
                                class: format!("install-step-button {state_class}"),
                                r#type: "button",
                                disabled: is_installing() || !install_step_accessible(
                                    step,
                                    admin_ready,
                                    general_ready,
                                    storage_ready,
                                ),
                                onclick: move |_| current_step.set(step),
                                div { class: "install-step-index", "{install_step_index_badge(step, index + 1, current_step_value, admin_ready, general_ready, storage_ready, review_ready)}" }
                                div { class: "install-step-copy",
                                    strong { "{step.label()}" }
                                    small { "{install_step_state_text(step, current_step_value, admin_ready, general_ready, storage_ready, review_ready)}" }
                                }
                            }
                        }
                    })}
                }

                section { class: "settings-card install-stage-card",
                    div { class: "settings-panel-head install-stage-head",
                        div {
                            h2 { class: "settings-panel-title", "{current_step_value.title()}" }
                        }
                    }

                    div { class: "install-stage-body",
                        if current_step_value == InstallWizardStep::Admin {
                            div { class: "settings-stack",
                                div { class: "settings-subcard install-env-panel",
                                    div { class: "settings-panel-badges",
                                        h3 { "部署环境" }
                                        span { class: "stat-pill", "只读环境信息" }
                                    }
                                    div { class: "install-env-inline",
                                        div { class: "install-env-inline-item",
                                            span { class: "install-env-inline-label", "数据库" }
                                            strong { class: "install-env-inline-value", "{database_label}" }
                                            span { class: database_status_class, "{database_status}" }
                                            code { class: "install-env-inline-code", "{database_connection}" }
                                        }
                                        div { class: "install-env-inline-item",
                                            span { class: "install-env-inline-label", "缓存" }
                                            strong { class: "install-env-inline-value", "{cache_label}" }
                                            span { class: cache_status_class, "{cache_status}" }
                                            code { class: "install-env-inline-code", "{cache_connection}" }
                                        }
                                    }
                                }

                                if !runtime_error.is_empty() {
                                    div { class: "settings-banner settings-banner-warning",
                                        "运行环境异常：{runtime_error}"
                                    }
                                }

                                div { class: "settings-subcard",
                                    h3 { "管理员账号" }
                                    div { class: "settings-grid",
                                        label { class: "settings-field settings-field-full",
                                            span { "管理员邮箱（必填）" }
                                            input {
                                                class: if admin_email_error.is_some() { "is-invalid" } else { "" },
                                                r#type: "email",
                                                placeholder: "admin@example.com",
                                                value: "{admin_email_value}",
                                                oninput: move |event| admin_email.set(event.value()),
                                                disabled: is_installing(),
                                                autocomplete: "email",
                                            }
                                            if let Some(message) = admin_email_error.clone() {
                                                small { class: "settings-field-hint settings-field-hint-error", "{message}" }
                                            }
                                        }

                                        label { class: "settings-field",
                                            span { "管理员密码（必填）" }
                                            div { class: "install-password-field",
                                                input {
                                                    class: if admin_password_error.is_some() { "is-invalid" } else { "" },
                                                    r#type: if show_admin_password() { "text" } else { "password" },
                                                    placeholder: "至少 12 个字符",
                                                    value: "{admin_password_value}",
                                                    oninput: move |event| admin_password.set(event.value()),
                                                    disabled: is_installing(),
                                                    autocomplete: "new-password",
                                                }
                                                button {
                                                    class: "btn btn-ghost install-password-toggle",
                                                    r#type: "button",
                                                    onclick: move |_| show_admin_password.toggle(),
                                                    disabled: is_installing(),
                                                    if show_admin_password() {
                                                        "隐藏"
                                                    } else {
                                                        "显示"
                                                    }
                                                }
                                            }
                                            if let Some(message) = admin_password_error.clone() {
                                                small { class: "settings-field-hint settings-field-hint-error", "{message}" }
                                            }
                                        }

                                        label { class: "settings-field",
                                            span { "确认密码（必填）" }
                                            div { class: "install-password-field",
                                                input {
                                                    class: if confirm_password_error.is_some() { "is-invalid" } else { "" },
                                                    r#type: if show_confirm_password() { "text" } else { "password" },
                                                    placeholder: "再次输入同一密码",
                                                    value: "{confirm_password_value}",
                                                    oninput: move |event| confirm_password.set(event.value()),
                                                    disabled: is_installing(),
                                                    autocomplete: "new-password",
                                                }
                                                button {
                                                    class: "btn btn-ghost install-password-toggle",
                                                    r#type: "button",
                                                    onclick: move |_| show_confirm_password.toggle(),
                                                    disabled: is_installing(),
                                                    if show_confirm_password() {
                                                        "隐藏"
                                                    } else {
                                                        "显示"
                                                    }
                                                }
                                            }
                                            if let Some(message) = confirm_password_error.clone() {
                                                small { class: "settings-field-hint settings-field-hint-error", "{message}" }
                                            }
                                        }
                                    }
                                }
                            }
                        } else if current_step_value == InstallWizardStep::General {
                            div { class: "settings-stack",
                                div { class: "settings-subcard install-compact-subcard",
                                    h3 { "品牌图标" }
                                    div { class: "settings-grid settings-grid-single",
                                        label { class: "settings-field settings-field-full",
                                            span { "网站图标（可选）" }
                                            input {
                                                r#type: "file",
                                                accept: ".ico,image/png,image/svg+xml,image/webp,image/jpeg,image/x-icon,image/vnd.microsoft.icon",
                                                onchange: handle_pick_favicon,
                                                disabled: is_installing(),
                                            }
                                        }
                                        if has_custom_favicon {
                                            p { class: "install-file-meta settings-field-full",
                                                "已选择图标：{selected_favicon_name}"
                                            }
                                        } else {
                                            p { class: "install-file-meta settings-field-full",
                                                "当前未选择图标"
                                            }
                                        }
                                    }
                                }

                                {render_general_fields_compact(form, is_installing())}
                            }
                        } else if current_step_value == InstallWizardStep::Storage {
                            div { class: "settings-stack",
                                div { class: "settings-subcard install-compact-subcard",
                                    h3 { "存储方案" }
                                    div { class: "settings-grid settings-grid-single",
                                        label { class: "settings-field",
                                            span { "存储后端（必填）" }
                                            select {
                                                value: "{(form.storage_backend)().as_str()}",
                                                onchange: move |event| {
                                                    (form.storage_backend).set(StorageBackendKind::parse(&event.value()));
                                                    last_tested_s3_request.set(None);
                                                    s3_test_feedback.set(String::new());
                                                    s3_test_feedback_tone.set(S3TestFeedbackTone::Neutral);
                                                    storage_browser_open.set(false);
                                                    storage_browser_error.set(String::new());
                                                },
                                                disabled: is_installing() || is_testing_s3(),
                                                option { value: StorageBackendKind::Unknown.as_str(), "请选择存储后端" }
                                                option { value: StorageBackendKind::Local.as_str(), "本地存储" }
                                                option { value: StorageBackendKind::S3.as_str(), "对象存储（兼容 S3）" }
                                            }
                                        }
                                    }
                                }

                                if (form.storage_backend)() == StorageBackendKind::Local {
                                    div { class: "settings-subcard install-compact-subcard",
                                        h3 { "本地存储" }
                                        div { class: "settings-grid settings-grid-single",
                                            div { class: "settings-field settings-field-full",
                                                span { "本地存储路径（必填）" }
                                                div { class: "install-path-picker",
                                                    input {
                                                        class: "install-path-input",
                                                        r#type: "text",
                                                        value: "{(form.local_storage_path)()}",
                                                        readonly: true,
                                                        disabled: true,
                                                    }
                                                    button {
                                                        class: "btn btn-ghost",
                                                        r#type: "button",
                                                        disabled: is_installing() || is_testing_s3() || storage_browser_loading(),
                                                        onclick: move |_| {
                                                            storage_browser_open.set(true);
                                                            load_install_storage_directories(
                                                                open_browser_service.clone(),
                                                                storage_browser_loading,
                                                                storage_browser_error,
                                                                storage_browser_current_path,
                                                                storage_browser_parent_path,
                                                                storage_browser_directories,
                                                                (form.local_storage_path)(),
                                                            );
                                                        },
                                                        if storage_browser_loading() { "读取中..." } else { "浏览" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else if form.is_s3_backend() {
                                    div { class: "settings-subcard install-compact-subcard",
                                        h3 { "对象存储（兼容 S3）" }
                                        div { class: "settings-grid",
                                            {render_s3_fields_compact(form, is_installing() || is_testing_s3())}
                                        }
                                        div { class: "settings-actions",
                                            button {
                                                class: "btn btn-primary",
                                                r#type: "button",
                                                onclick: handle_test_s3,
                                                disabled: is_installing() || is_testing_s3(),
                                                if is_testing_s3() {
                                                    "测试中..."
                                                } else {
                                                    "测试 S3 连通性"
                                                }
                                            }
                                        }
                                        if form.is_s3_backend() && !s3_test_confirmed {
                                            div { class: "settings-banner settings-banner-warning",
                                                "当前对象存储配置尚未验证，请先完成连通性测试。"
                                            }
                                        } else if !s3_test_feedback().is_empty() {
                                            div {
                                                class: match s3_test_feedback_tone() {
                                                    S3TestFeedbackTone::Success => "settings-banner settings-banner-success",
                                                    S3TestFeedbackTone::Error => "error-banner",
                                                    S3TestFeedbackTone::Neutral => "settings-banner settings-banner-warning",
                                                },
                                                "{s3_test_feedback()}"
                                            }
                                        }
                                    }
                                }

                                if storage_browser_open() {
                                    Modal {
                                        title: "选择本地存储目录".to_string(),
                                        content_class: "storage-browser-modal-shell".to_string(),
                                        on_close: move |_| storage_browser_open.set(false),
                                        div { class: "install-path-browser",
                                            div { class: "install-path-browser-head",
                                                code { class: "install-path-browser-current", "{storage_browser_current_path()}" }
                                                div { class: "install-path-browser-toolbar",
                                                    button {
                                                        class: "btn btn-ghost",
                                                        r#type: "button",
                                                        disabled: is_installing() || is_testing_s3() || storage_browser_loading() || storage_browser_parent_path().is_none(),
                                                        onclick: move |_| {
                                                            if let Some(parent_path) = storage_browser_parent_path() {
                                                                load_install_storage_directories(
                                                                    browse_parent_service.clone(),
                                                                    storage_browser_loading,
                                                                    storage_browser_error,
                                                                    storage_browser_current_path,
                                                                    storage_browser_parent_path,
                                                                    storage_browser_directories,
                                                                    parent_path,
                                                                );
                                                            }
                                                        },
                                                        "上一级"
                                                    }
                                                    button {
                                                        class: "btn btn-primary",
                                                        r#type: "button",
                                                        disabled: is_installing() || is_testing_s3(),
                                                        onclick: move |_| {
                                                            (form.local_storage_path).set(storage_browser_current_path());
                                                            storage_browser_open.set(false);
                                                        },
                                                        "选择当前文件夹"
                                                    }
                                                }
                                            }
                                            div { class: "install-path-browser-panel",
                                                if !storage_browser_error().is_empty() {
                                                    p { class: "install-path-browser-error", "{storage_browser_error()}" }
                                                } else if storage_browser_loading() {
                                                    p { class: "install-path-browser-empty", "正在读取目录..." }
                                                } else if storage_browser_directories().is_empty() {
                                                    p { class: "install-path-browser-empty", "当前目录下没有可继续展开的子目录。" }
                                                } else {
                                                    div { class: "install-path-browser-list",
                                                        {browser_directories.iter().map(|entry| {
                                                            let entry_path = entry.path.clone();
                                                            let entry_name = entry.name.clone();
                                                            let browse_entry_service = install_service.clone();
                                                            rsx! {
                                                                button {
                                                                    key: "{entry_path}",
                                                                    class: "install-path-browser-item",
                                                                    r#type: "button",
                                                                    disabled: is_installing() || is_testing_s3() || storage_browser_loading(),
                                                                    onclick: move |_| {
                                                                        load_install_storage_directories(
                                                                            browse_entry_service.clone(),
                                                                            storage_browser_loading,
                                                                            storage_browser_error,
                                                                            storage_browser_current_path,
                                                                            storage_browser_parent_path,
                                                                            storage_browser_directories,
                                                                            entry_path.clone(),
                                                                        );
                                                                    },
                                                                    span { class: "install-path-browser-folder" }
                                                                    span { class: "install-path-browser-name", "{entry_name}" }
                                                                }
                                                            }
                                                        })}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            div { class: "settings-stack",
                                div { class: "settings-subcard install-compact-subcard",
                                    h3 { "最终摘要" }
                                    div { class: "install-review-list install-review-card",
                                        {render_install_review_row("管理员", summary_or_pending(admin_email_value.clone()))}
                                        {render_install_review_row("站点名称", site_name_summary.clone())}
                                        {render_install_review_row("站点图标", favicon_summary.clone())}
                                        {render_install_review_row("邮件服务", mail_summary.clone())}
                                        {render_install_review_row("存储后端", storage_summary.clone())}
                                        {render_install_review_row("数据库来源", format!("{database_label} / {database_status}"))}
                                        {render_install_review_row("缓存来源", format!("{cache_label} / {cache_status}"))}
                                    }
                                }

                                div { class: "settings-subcard install-compact-subcard",
                                    h3 { "提交前检查" }
                                    div { class: "settings-checklist install-check-card",
                                        {render_install_check_item("管理员", "邮箱格式正确，密码满足当前最小长度并且两次输入一致", admin_ready)}
                                        {render_install_check_item("站点", "网站名称已填写，品牌信息可以直接对外展示", site_ready)}
                                        {render_install_check_item("邮件", "邮件服务已关闭，或所有 SMTP 与链接参数完整", mail_ready)}
                                        {render_install_check_item("存储", "本地路径已选择，或对象存储字段完整且已通过连通性测试", storage_ready)}
                                    }
                                }
                            }
                        }
                    }

                    if !success_message().is_empty() {
                        div { class: "settings-banner settings-banner-success", "{success_message()}" }
                    }
                    if !error_message().is_empty() {
                        div { class: "error-banner", "{error_message()}" }
                    }

                    div { class: "install-stage-actions",
                        div { class: "install-stage-actions-group",
                            button {
                                class: "btn btn-ghost install-back-button",
                                r#type: "button",
                                disabled: is_installing() || is_testing_s3() || prev_step.is_none(),
                                onclick: move |_| {
                                    if let Some(step) = prev_step {
                                        current_step.set(step);
                                    }
                                },
                                "上一步"
                            }

                            button {
                                class: if is_review_step {
                                    "btn btn-primary install-submit-button"
                                } else {
                                    "btn btn-primary install-continue-button"
                                },
                                r#type: "button",
                                disabled: is_installing()
                                    || is_testing_s3()
                                    || (!is_review_step
                                        && (!current_step_ready || next_step.is_none()))
                                    || (is_review_step && !review_ready),
                                onclick: handle_primary_action,
                                "{primary_action_label}"
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn favicon_file_to_data_url(file: FileData) -> Result<String, String> {
    let mime = infer_favicon_mime(&file);
    let bytes = file
        .read_bytes()
        .await
        .map_err(|err| format!("读取网站图标失败: {}", err))?;
    if bytes.is_empty() {
        return Err("网站图标内容为空".to_string());
    }

    Ok(format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

fn infer_favicon_mime(file: &FileData) -> &'static str {
    if let Some(content_type) = file.content_type() {
        match content_type.trim().to_ascii_lowercase().as_str() {
            "image/x-icon" | "image/vnd.microsoft.icon" | "image/ico" => {
                return "image/x-icon";
            }
            "image/png" => return "image/png",
            "image/svg+xml" => return "image/svg+xml",
            "image/webp" => return "image/webp",
            "image/jpeg" | "image/jpg" => return "image/jpeg",
            _ => {}
        }
    }

    let filename = file.name().to_ascii_lowercase();
    if filename.ends_with(".ico") {
        "image/x-icon"
    } else if filename.ends_with(".svg") {
        "image/svg+xml"
    } else if filename.ends_with(".webp") {
        "image/webp"
    } else if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "image/png"
    }
}

fn install_step_index(step: InstallWizardStep) -> usize {
    INSTALL_WIZARD_STEPS
        .iter()
        .position(|candidate| *candidate == step)
        .unwrap_or(0)
}

fn install_step_state_class(
    step: InstallWizardStep,
    current_step: InstallWizardStep,
    admin_ready: bool,
    general_ready: bool,
    storage_ready: bool,
    review_ready: bool,
) -> &'static str {
    if step == current_step {
        "is-active"
    } else if install_step_complete(
        step,
        admin_ready,
        general_ready,
        storage_ready,
        review_ready,
    ) {
        "is-done"
    } else {
        "is-pending"
    }
}

fn install_step_state_text(
    step: InstallWizardStep,
    current_step: InstallWizardStep,
    admin_ready: bool,
    general_ready: bool,
    storage_ready: bool,
    review_ready: bool,
) -> &'static str {
    if step == current_step {
        "进行中"
    } else if install_step_complete(
        step,
        admin_ready,
        general_ready,
        storage_ready,
        review_ready,
    ) {
        "已完成"
    } else {
        "待完成"
    }
}

fn install_step_index_badge(
    step: InstallWizardStep,
    index: usize,
    current_step: InstallWizardStep,
    admin_ready: bool,
    general_ready: bool,
    storage_ready: bool,
    review_ready: bool,
) -> String {
    if step != current_step
        && install_step_complete(
            step,
            admin_ready,
            general_ready,
            storage_ready,
            review_ready,
        )
    {
        "✓".to_string()
    } else {
        index.to_string()
    }
}

fn install_step_complete(
    step: InstallWizardStep,
    admin_ready: bool,
    general_ready: bool,
    storage_ready: bool,
    review_ready: bool,
) -> bool {
    match step {
        InstallWizardStep::Admin => admin_ready,
        InstallWizardStep::General => general_ready,
        InstallWizardStep::Storage => storage_ready,
        InstallWizardStep::Review => review_ready,
    }
}

fn install_step_accessible(
    step: InstallWizardStep,
    admin_ready: bool,
    general_ready: bool,
    storage_ready: bool,
) -> bool {
    match step {
        InstallWizardStep::Admin => true,
        InstallWizardStep::General => admin_ready,
        InstallWizardStep::Storage => admin_ready && general_ready,
        InstallWizardStep::Review => admin_ready && general_ready && storage_ready,
    }
}

fn install_admin_email_error(email: &str) -> Option<String> {
    let email = email.trim();
    if email.is_empty() || email.contains('@') {
        None
    } else {
        Some("请输入有效的管理员邮箱".to_string())
    }
}

fn install_admin_password_error(password: &str) -> Option<String> {
    let password = password.trim();
    if password.is_empty() || password.len() >= MIN_ADMIN_PASSWORD_LENGTH {
        None
    } else {
        Some(format!(
            "管理员密码至少需要 {} 个字符",
            MIN_ADMIN_PASSWORD_LENGTH
        ))
    }
}

fn install_admin_confirm_password_error(password: &str, confirm: &str) -> Option<String> {
    if password.is_empty() || confirm.is_empty() || password == confirm {
        None
    } else {
        Some("两次输入的管理员密码不一致".to_string())
    }
}

fn install_admin_submit_error(email: &str, password: &str, confirm: &str) -> Option<String> {
    let email = email.trim();
    if email.is_empty() {
        Some("请填写管理员邮箱".to_string())
    } else if password.trim().is_empty() {
        Some("请填写管理员密码".to_string())
    } else if password.trim().len() < MIN_ADMIN_PASSWORD_LENGTH {
        Some(format!(
            "管理员密码至少需要 {} 个字符",
            MIN_ADMIN_PASSWORD_LENGTH
        ))
    } else if password != confirm {
        Some("两次输入的管理员密码不一致".to_string())
    } else {
        None
    }
}

fn install_admin_ready(email: &str, password: &str, confirm: &str) -> bool {
    email.trim().contains('@') && install_admin_submit_error(email, password, confirm).is_none()
}

fn install_mail_ready(form: SettingsFormState) -> bool {
    if !(form.mail_enabled)() {
        return true;
    }

    let smtp_host = (form.mail_smtp_host)().trim().to_string();
    let smtp_port = (form.mail_smtp_port)().trim().to_string();
    let smtp_user = (form.mail_smtp_user)().trim().to_string();
    let smtp_password = (form.mail_smtp_password)().trim().to_string();
    let from_email = (form.mail_from_email)().trim().to_string();
    let link_base_url = (form.mail_link_base_url)().trim().to_string();
    let password_ready =
        !smtp_password.is_empty() || ((form.mail_smtp_password_set)() && !smtp_user.is_empty());

    !smtp_host.is_empty()
        && !from_email.is_empty()
        && !link_base_url.is_empty()
        && smtp_port
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)
            .is_some()
        && (smtp_user.is_empty() == password_ready)
}

fn install_storage_ready(form: SettingsFormState, s3_test_confirmed: bool) -> bool {
    match (form.storage_backend)() {
        StorageBackendKind::Unknown => false,
        StorageBackendKind::Local => !(form.local_storage_path)().trim().is_empty(),
        StorageBackendKind::S3 => form.is_s3_configuration_complete() && s3_test_confirmed,
    }
}

fn is_current_install_s3_request_confirmed(
    form: SettingsFormState,
    last_tested_request: Option<TestS3StorageConfigRequest>,
) -> bool {
    last_tested_request.is_some_and(|tested| tested == form.build_s3_test_request())
}

fn render_install_check_item(
    title: &'static str,
    description: &'static str,
    done: bool,
) -> Element {
    rsx! {
        article { class: if done {
            "settings-checklist-item is-done"
        } else {
            "settings-checklist-item is-pending"
        },
            div { class: "settings-checklist-indicator",
                if done { "✓" } else { "·" }
            }
            div { class: "settings-checklist-copy",
                strong { "{title}" }
                p { "{description}" }
            }
        }
    }
}

fn render_install_review_row(title: &'static str, value: String) -> Element {
    rsx! {
        div { class: "install-review-row",
            span { class: "install-review-label", "{title}" }
            strong { class: "install-review-value", "{value}" }
        }
    }
}

fn load_install_storage_directories(
    install_service: InstallService,
    mut storage_browser_loading: Signal<bool>,
    mut storage_browser_error: Signal<String>,
    mut storage_browser_current_path: Signal<String>,
    mut storage_browser_parent_path: Signal<Option<String>>,
    mut storage_browser_directories: Signal<Vec<StorageDirectoryEntry>>,
    requested_path: String,
) {
    let requested_path = requested_path.trim().to_string();
    let requested_path = if requested_path.is_empty() {
        DEFAULT_INSTALL_STORAGE_BROWSER_PATH.to_string()
    } else {
        requested_path
    };

    spawn(async move {
        storage_browser_loading.set(true);
        storage_browser_error.set(String::new());

        let response = install_service
            .browse_storage_directories(Some(requested_path.as_str()))
            .await;

        match response {
            Ok(response) => {
                storage_browser_current_path.set(response.current_path);
                storage_browser_parent_path.set(response.parent_path);
                storage_browser_directories.set(response.directories);
            }
            Err(error) => {
                storage_browser_error.set(format!("读取目录失败：{}", error));
            }
        }

        storage_browser_loading.set(false);
    });
}

fn summary_or_pending(value: String) -> String {
    let value = value.trim().to_string();
    if value.is_empty() {
        "待填写".to_string()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_admin_password_error_is_shown_only_for_short_non_empty_passwords() {
        assert_eq!(install_admin_password_error(""), None);
        assert_eq!(
            install_admin_password_error("short"),
            Some("管理员密码至少需要 12 个字符".to_string())
        );
        assert_eq!(install_admin_password_error("Password123!"), None);
    }

    #[test]
    fn install_admin_confirm_password_error_waits_for_both_values_and_checks_match() {
        assert_eq!(
            install_admin_confirm_password_error("", "Password123!"),
            None
        );
        assert_eq!(
            install_admin_confirm_password_error("Password123!", ""),
            None
        );
        assert_eq!(
            install_admin_confirm_password_error("Password123!", "Password321!"),
            Some("两次输入的管理员密码不一致".to_string())
        );
        assert_eq!(
            install_admin_confirm_password_error("Password123!", "Password123!"),
            None
        );
    }

    #[test]
    fn install_admin_submit_error_matches_install_requirements() {
        assert_eq!(
            install_admin_submit_error("", "Password123!", "Password123!"),
            Some("请填写管理员邮箱".to_string())
        );
        assert_eq!(
            install_admin_submit_error("admin@example.com", "", ""),
            Some("请填写管理员密码".to_string())
        );
        assert_eq!(
            install_admin_submit_error("admin@example.com", "short", "short"),
            Some("管理员密码至少需要 12 个字符".to_string())
        );
        assert_eq!(
            install_admin_submit_error("admin@example.com", "Password123!", "Password321!"),
            Some("两次输入的管理员密码不一致".to_string())
        );
        assert_eq!(
            install_admin_submit_error("admin@example.com", "Password123!", "Password123!"),
            None
        );
    }

    #[test]
    fn install_admin_ready_requires_valid_email_and_matching_strong_password() {
        assert!(!install_admin_ready(
            "invalid-email",
            "Password123!",
            "Password123!"
        ));
        assert!(!install_admin_ready("admin@example.com", "short", "short"));
        assert!(install_admin_ready(
            "admin@example.com",
            "Password123!",
            "Password123!"
        ));
    }

    #[test]
    fn initial_local_storage_path_keeps_empty_value_until_user_selects_directory() {
        let config = AdminSettingsConfig {
            site_name: "Vansour Image".to_string(),
            storage_backend: StorageBackendKind::Local,
            local_storage_path: "   ".to_string(),
            mail_enabled: false,
            mail_smtp_host: String::new(),
            mail_smtp_port: 587,
            mail_smtp_user: None,
            mail_smtp_password_set: false,
            mail_from_email: String::new(),
            mail_from_name: String::new(),
            mail_link_base_url: String::new(),
            s3_endpoint: None,
            s3_region: None,
            s3_bucket: None,
            s3_prefix: None,
            s3_access_key: None,
            s3_secret_key_set: false,
            s3_force_path_style: true,
            restart_required: false,
        };

        assert!(initial_local_storage_path(&config).is_empty());
    }

    #[test]
    fn initial_site_name_clears_default_brand_name() {
        let default_config = AdminSettingsConfig {
            site_name: "Vansour Image".to_string(),
            storage_backend: StorageBackendKind::Local,
            local_storage_path: String::new(),
            mail_enabled: false,
            mail_smtp_host: String::new(),
            mail_smtp_port: 587,
            mail_smtp_user: None,
            mail_smtp_password_set: false,
            mail_from_email: String::new(),
            mail_from_name: String::new(),
            mail_link_base_url: String::new(),
            s3_endpoint: None,
            s3_region: None,
            s3_bucket: None,
            s3_prefix: None,
            s3_access_key: None,
            s3_secret_key_set: false,
            s3_force_path_style: true,
            restart_required: false,
        };
        let custom_config = AdminSettingsConfig {
            site_name: "Acme Images".to_string(),
            ..default_config.clone()
        };

        assert!(initial_site_name(&default_config).is_empty());
        assert_eq!(initial_site_name(&custom_config), "Acme Images".to_string());
    }

    #[test]
    fn install_step_accessible_requires_previous_steps_to_finish() {
        assert!(install_step_accessible(
            InstallWizardStep::Admin,
            false,
            false,
            false
        ));
        assert!(!install_step_accessible(
            InstallWizardStep::General,
            false,
            false,
            false
        ));
        assert!(!install_step_accessible(
            InstallWizardStep::Storage,
            true,
            false,
            false
        ));
        assert!(install_step_accessible(
            InstallWizardStep::Review,
            true,
            true,
            true
        ));
    }
}
