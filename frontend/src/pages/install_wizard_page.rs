use crate::app_context::{use_install_service, use_toast_store};
use crate::components::Modal;
use crate::pages::settings_page::{
    SettingsFormState, default_mail_link_base_url, render_general_fields_compact, render_s3_fields,
};
use crate::services::InstallService;
use crate::types::api::{
    AdminSettingsConfig, BootstrapStatusResponse, InstallBootstrapRequest,
    InstallBootstrapResponse, StorageDirectoryEntry,
};
use base64::Engine;
use dioxus::html::FileData;
use dioxus::prelude::*;

const MIN_ADMIN_PASSWORD_LENGTH: usize = 12;
const DEFAULT_INSTALL_LOCAL_STORAGE_PATH: &str = "/data/images";
const INSTALL_WIZARD_STEPS: [InstallWizardStep; 4] = [
    InstallWizardStep::Admin,
    InstallWizardStep::General,
    InstallWizardStep::Storage,
    InstallWizardStep::Review,
];

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
            Self::Admin => "部署环境",
            Self::General => "站点信息",
            Self::Storage => "存储后端",
            Self::Review => "最终确认",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Admin => "确认部署环境并创建首个管理员",
            Self::General => "完善站点信息与品牌标识",
            Self::Storage => "选择图片存储方案",
            Self::Review => "复核配置并初始化系统",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Admin => "先核对部署层注入的数据库与缓存，再创建首个管理员账户。",
            Self::General => "配置站点名称、图标和邮件服务。",
            Self::Storage => "选择本地存储或对象存储。",
            Self::Review => "最后统一检查关键配置。确认无误后系统会写入安装状态并自动登录管理员。",
        }
    }
}

fn initial_local_storage_path(config: &AdminSettingsConfig) -> String {
    let configured = config.local_storage_path.trim();
    if !configured.is_empty() {
        return configured.to_string();
    }

    DEFAULT_INSTALL_LOCAL_STORAGE_PATH.to_string()
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
        let initial = initial_config.site_name.clone();
        move || initial.clone()
    });
    let storage_backend = use_signal({
        let initial = initial_config.storage_backend.clone();
        move || initial.clone()
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
        let initial = initial_config.mail_smtp_port.to_string();
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
    };

    let mut admin_email = use_signal(String::new);
    let mut admin_password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut selected_favicon = use_signal(|| None::<FileData>);
    let mut current_step = use_signal(|| InstallWizardStep::Admin);
    let mut is_installing = use_signal(|| false);
    let mut error_message = use_signal(String::new);
    let mut success_message = use_signal(String::new);
    let mut storage_browser_open = use_signal(|| false);
    let storage_browser_loading = use_signal(|| false);
    let mut storage_browser_error = use_signal(String::new);
    let storage_browser_current_path =
        use_signal(|| DEFAULT_INSTALL_LOCAL_STORAGE_PATH.to_string());
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
    let mut handle_install = move || {
        if is_installing() {
            return;
        }

        let email = admin_email().trim().to_string();
        let password = admin_password();
        let confirm = confirm_password();
        if let Some(message) =
            install_admin_submit_error(email.as_str(), password.as_str(), confirm.as_str())
        {
            error_message.set(message.clone());
            toast_store.show_error(message);
            return;
        }
        if let Err(message) = form.validate() {
            error_message.set(message.clone());
            toast_store.show_error(message);
            return;
        }

        let install_service = install_service_for_submit.clone();
        let toast_store = toast_store.clone();
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
                    let message = if response.config.restart_required {
                        "安装完成，管理员已登录；存储配置需要重启服务后生效".to_string()
                    } else {
                        "安装完成，已自动登录管理员账户".to_string()
                    };
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
    let storage_ready = install_storage_ready(form);
    let review_ready = admin_ready && general_ready && storage_ready;
    let storage_summary = if form.is_s3_backend() {
        "对象存储".to_string()
    } else {
        "本地目录".to_string()
    };
    let mail_summary = if !(form.mail_enabled)() {
        "未启用".to_string()
    } else if mail_ready {
        "已启用".to_string()
    } else {
        "待补全".to_string()
    };
    let environment_source = if bootstrap_status.mode == "runtime" {
        "Docker Compose / 环境变量"
    } else {
        "Bootstrap 配置文件"
    };
    let environment_source_detail = format!("来源：{environment_source}");
    let environment_management_note = if bootstrap_status.mode == "runtime" {
        "数据库与缓存已经在部署层写入，这里只做只读展示。"
    } else {
        "数据库与缓存来自 bootstrap 配置文件，这里只做只读展示。"
    };
    let database_label = bootstrap_database_kind_label(&bootstrap_status.database_kind).to_string();
    let database_status = if bootstrap_status.database_configured {
        "已接入".to_string()
    } else {
        "未配置".to_string()
    };
    let database_connection = bootstrap_status
        .database_url_masked
        .clone()
        .unwrap_or_else(|| "未检测到数据库连接".to_string());
    let cache_label = if bootstrap_status.cache_configured {
        "Redis 外部缓存".to_string()
    } else {
        "无外部缓存".to_string()
    };
    let cache_status = if bootstrap_status.cache_configured {
        "已接入".to_string()
    } else {
        "已关闭".to_string()
    };
    let cache_connection = bootstrap_status
        .cache_url_masked
        .clone()
        .unwrap_or_else(|| "未配置 REDIS_URL".to_string());
    let runtime_error = bootstrap_status.runtime_error.clone().unwrap_or_default();
    let site_name_summary = summary_or_pending((form.site_name)());

    let current_step_value = current_step();
    let current_step_index = install_step_index(current_step_value);
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
            "完成安装".to_string()
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

    rsx! {
        div { class: "dashboard-page settings-page install-page install-page-wizard",
            div { class: "install-wizard-shell",
                section { class: "page-hero settings-hero settings-hero-rich install-hero",
                    div { class: "settings-hero-main settings-hero-main-stack install-hero-main",
                        div {
                            h1 { "安装向导" }
                            p { class: "settings-hero-copy",
                                "按顺序完成部署环境、站点信息和存储后端配置。"
                            }
                        }
                        div { class: "settings-pill-row",
                            span { class: "stat-pill stat-pill-active", "当前步骤 {current_step_index + 1}/{total_steps}" }
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
                        let state_text = install_step_state_text(
                            step,
                            admin_ready,
                            site_ready,
                            mail_ready,
                            storage_ready,
                            review_ready,
                        );

                        rsx! {
                            button {
                                class: format!("install-step-button {state_class}"),
                                r#type: "button",
                                disabled: is_installing(),
                                onclick: move |_| current_step.set(step),
                                div { class: "install-step-index", "{index + 1}" }
                                div { class: "install-step-copy",
                                    strong { "{step.label()}" }
                                    small { "{state_text}" }
                                }
                            }
                        }
                    })}
                }

                section { class: "settings-card install-stage-card",
                    div { class: "settings-panel-head install-stage-head",
                        div {
                            h2 { class: "settings-panel-title", "{current_step_value.title()}" }
                            p { class: "settings-panel-copy", "{current_step_value.description()}" }
                        }
                    }

                    div { class: "install-stage-body",
                        if current_step_value == InstallWizardStep::Admin {
                            div { class: "settings-stack",
                                div { class: "settings-subcard install-env-panel",
                                    div { class: "install-section-header",
                                        div {
                                            h3 { "已检测到的运行环境" }
                                            p { class: "settings-section-copy",
                                                "{environment_management_note}"
                                            }
                                        }
                                        span { class: "stat-pill", "只读" }
                                    }
                                    div { class: "install-env-grid",
                                        article { class: "install-env-card",
                                            div { class: "install-env-head",
                                                p { class: "install-env-label", "数据库" }
                                                span {
                                                    class: if bootstrap_status.database_configured {
                                                        "stat-pill stat-pill-active"
                                                    } else {
                                                        "stat-pill"
                                                    },
                                                    "{database_status}"
                                                }
                                            }
                                            p { class: "install-env-kind", "{database_label}" }
                                            input {
                                                class: "install-env-input",
                                                r#type: "text",
                                                value: "{database_connection}",
                                                disabled: true,
                                            }
                                            div { class: "install-env-meta",
                                                span { class: "install-env-hint", "{environment_source_detail}" }
                                            }
                                        }
                                        article { class: "install-env-card",
                                            div { class: "install-env-head",
                                                p { class: "install-env-label", "缓存" }
                                                span {
                                                    class: if bootstrap_status.cache_configured {
                                                        "stat-pill stat-pill-active"
                                                    } else {
                                                        "stat-pill"
                                                    },
                                                    "{cache_status}"
                                                }
                                            }
                                            p { class: "install-env-kind", "{cache_label}" }
                                            input {
                                                class: "install-env-input",
                                                r#type: "text",
                                                value: "{cache_connection}",
                                                disabled: true,
                                            }
                                            div { class: "install-env-meta",
                                                span { class: "install-env-hint", "{environment_source_detail}" }
                                            }
                                        }
                                    }
                                }

                                if !runtime_error.is_empty() {
                                    div { class: "settings-banner settings-banner-warning",
                                        "当前运行环境报告异常：{runtime_error}。建议先修正数据库或缓存连通性，再继续安装。"
                                    }
                                }

                                div { class: "settings-subcard",
                                    h3 { "创建首个管理员" }
                                    p { class: "settings-section-copy",
                                        "安装完成后会自动以该管理员身份进入后台。密码至少 {MIN_ADMIN_PASSWORD_LENGTH} 位。"
                                    }
                                    div { class: "settings-grid",
                                        label { class: "settings-field settings-field-full",
                                            span { "管理员邮箱" }
                                            input {
                                                class: if admin_email_error.is_some() { "is-invalid" } else { "" },
                                                r#type: "email",
                                                value: "{admin_email_value}",
                                                oninput: move |event| admin_email.set(event.value()),
                                                disabled: is_installing(),
                                            }
                                            if let Some(message) = admin_email_error.clone() {
                                                small { class: "settings-field-hint settings-field-hint-error", "{message}" }
                                            }
                                        }

                                        label { class: "settings-field",
                                            span { "管理员密码" }
                                            input {
                                                class: if admin_password_error.is_some() { "is-invalid" } else { "" },
                                                r#type: "password",
                                                value: "{admin_password_value}",
                                                oninput: move |event| admin_password.set(event.value()),
                                                disabled: is_installing(),
                                            }
                                            if let Some(message) = admin_password_error.clone() {
                                                small { class: "settings-field-hint settings-field-hint-error", "{message}" }
                                            } else {
                                                small { class: "settings-field-hint",
                                                    "至少 12 位，建议包含大小写字母、数字与符号。"
                                                }
                                            }
                                        }

                                        label { class: "settings-field",
                                            span { "确认密码" }
                                            input {
                                                class: if confirm_password_error.is_some() { "is-invalid" } else { "" },
                                                r#type: "password",
                                                value: "{confirm_password_value}",
                                                oninput: move |event| confirm_password.set(event.value()),
                                                disabled: is_installing(),
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
                                    p { class: "settings-section-copy",
                                        "可选，支持 ico / png / svg / webp / jpeg。"
                                    }
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
                                                "当前未上传图标，系统会先使用默认站点图标。"
                                            }
                                        }
                                    }
                                }

                                {render_general_fields_compact(form, is_installing())}
                            }
                        } else if current_step_value == InstallWizardStep::Storage {
                            div { class: "settings-stack",
                                div { class: "settings-subcard install-compact-subcard",
                                    h3 { "存储后端" }
                                    div { class: "settings-grid",
                                        label { class: "settings-field",
                                            span { "存储后端" }
                                            select {
                                                value: "{(form.storage_backend)()}",
                                                onchange: move |event| {
                                                    let next = event.value();
                                                    (form.storage_backend).set(next.clone());
                                                    if (form.local_storage_path)().trim().is_empty() {
                                                        (form.local_storage_path).set(DEFAULT_INSTALL_LOCAL_STORAGE_PATH.to_string());
                                                    }
                                                    storage_browser_open.set(false);
                                                    storage_browser_error.set(String::new());
                                                },
                                                disabled: is_installing(),
                                                option { value: "local", "本地存储" }
                                                option { value: "s3", "对象存储（S3）" }
                                            }
                                        }

                                        if !form.is_s3_backend() {
                                            div { class: "settings-field settings-field-full",
                                                span { "本地存储路径" }
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
                                                        disabled: is_installing() || storage_browser_loading(),
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
                                                        if storage_browser_loading() { "读取中..." } else { "选择文件夹" }
                                                    }
                                                }
                                                if storage_browser_open() {
                                                    Modal {
                                                        title: "选择本地存储目录".to_string(),
                                                        content_class: "storage-browser-modal-shell".to_string(),
                                                        on_close: move |_| storage_browser_open.set(false),
                                                        div { class: "install-path-browser",
                                                            div { class: "install-path-browser-summary",
                                                                div {
                                                                    p { class: "install-path-browser-label", "当前目录" }
                                                                    code { class: "install-path-browser-current", "{storage_browser_current_path()}" }
                                                                }
                                                                p { class: "install-path-browser-meta",
                                                                    if storage_browser_loading() {
                                                                        "正在读取目录..."
                                                                    } else {
                                                                        "{browser_directories.len()} 个子目录"
                                                                    }
                                                                }
                                                            }
                                                            div { class: "install-path-browser-toolbar",
                                                                button {
                                                                    class: "btn btn-ghost",
                                                                    r#type: "button",
                                                                    disabled: is_installing() || storage_browser_loading() || storage_browser_parent_path().is_none(),
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
                                                                    disabled: is_installing(),
                                                                    onclick: move |_| {
                                                                        (form.local_storage_path).set(storage_browser_current_path());
                                                                        storage_browser_open.set(false);
                                                                    },
                                                                    "选择当前文件夹"
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
                                                                                    disabled: is_installing() || storage_browser_loading(),
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
                                        }
                                    }
                                }

                                if form.is_s3_backend() {
                                    div { class: "settings-subcard install-compact-subcard",
                                        h3 { "对象存储" }
                                        div { class: "settings-grid",
                                            {render_s3_fields(form, is_installing())}
                                        }
                                    }
                                }
                            }
                        } else {
                            div { class: "settings-stack",
                                div { class: "settings-subcard install-compact-subcard",
                                    h3 { "安装摘要" }
                                    div { class: "install-review-list",
                                        {render_install_review_row("管理员", summary_or_pending(admin_email_value.clone()))}
                                        {render_install_review_row("站点名称", site_name_summary.clone())}
                                        {render_install_review_row("站点图标", favicon_summary.clone())}
                                        {render_install_review_row("邮件服务", mail_summary.clone())}
                                        {render_install_review_row("存储后端", storage_summary.clone())}
                                    }
                                }

                                div { class: "settings-subcard install-compact-subcard",
                                    h3 { "提交前检查" }
                                    div { class: "settings-checklist",
                                        {render_install_check_item("管理员账户", "邮箱已填写且密码满足强度要求", admin_ready)}
                                        {render_install_check_item("站点识别", "站点名称已填写", site_ready)}
                                        {render_install_check_item("邮件配置", "关闭邮件，或邮件服务参数已补全", mail_ready)}
                                        {render_install_check_item("存储配置", "当前存储后端所需字段已补全", storage_ready)}
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
                                class: "btn btn-ghost",
                                r#type: "button",
                                disabled: is_installing() || prev_step.is_none(),
                                onclick: move |_| {
                                    if let Some(step) = prev_step {
                                        current_step.set(step);
                                    }
                                },
                                "上一步"
                            }

                            button {
                                class: "btn btn-primary",
                                r#type: "button",
                                disabled: is_installing() || (!is_review_step && next_step.is_none()) || (is_review_step && !review_ready),
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
    admin_ready: bool,
    site_ready: bool,
    mail_ready: bool,
    storage_ready: bool,
    review_ready: bool,
) -> String {
    match step {
        InstallWizardStep::Admin => {
            if admin_ready {
                "已就绪".to_string()
            } else {
                "待填写".to_string()
            }
        }
        InstallWizardStep::General => {
            if site_ready && mail_ready {
                "已就绪".to_string()
            } else if site_ready {
                "邮件待补全".to_string()
            } else {
                "待补全".to_string()
            }
        }
        InstallWizardStep::Storage => {
            if storage_ready {
                "已就绪".to_string()
            } else {
                "待补全".to_string()
            }
        }
        InstallWizardStep::Review => {
            if review_ready {
                "可提交".to_string()
            } else {
                "待检查".to_string()
            }
        }
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

fn bootstrap_database_kind_label(kind: &str) -> &'static str {
    if kind.eq_ignore_ascii_case("postgresql") || kind.eq_ignore_ascii_case("postgres") {
        "PostgreSQL"
    } else if kind.eq_ignore_ascii_case("mysql") {
        "MySQL / MariaDB"
    } else if kind.eq_ignore_ascii_case("sqlite") {
        "SQLite"
    } else {
        "未识别数据库"
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

fn install_storage_ready(form: SettingsFormState) -> bool {
    if !form.is_s3_backend() {
        return !(form.local_storage_path)().trim().is_empty();
    }

    !(form.local_storage_path)().trim().is_empty()
        && !(form.s3_endpoint)().trim().is_empty()
        && !(form.s3_region)().trim().is_empty()
        && !(form.s3_bucket)().trim().is_empty()
        && !(form.s3_access_key)().trim().is_empty()
        && ((form.s3_secret_key_set)() || !(form.s3_secret_key)().trim().is_empty())
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
    let requested_path = if requested_path.trim().is_empty() {
        DEFAULT_INSTALL_LOCAL_STORAGE_PATH.to_string()
    } else {
        requested_path
    };

    spawn(async move {
        storage_browser_loading.set(true);
        storage_browser_error.set(String::new());

        match install_service
            .browse_storage_directories(Some(requested_path.as_str()))
            .await
        {
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
}
