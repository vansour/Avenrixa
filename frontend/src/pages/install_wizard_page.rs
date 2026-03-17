mod page_controller;
mod summary;

use crate::components::Modal;
use crate::pages::settings_page::{render_general_fields_compact, render_s3_fields_compact};
use crate::types::api::{
    AdminSettingsConfig, BootstrapStatusResponse, InstallBootstrapResponse, StorageBackendKind,
};
use dioxus::prelude::*;

use page_controller::use_install_wizard_controller;
use summary::{
    INSTALL_WIZARD_STEPS, InstallWizardStep, S3TestFeedbackTone, current_install_mail_summary,
    current_install_storage_summary, install_admin_confirm_password_error,
    install_admin_email_error, install_admin_password_error, install_admin_ready,
    install_mail_ready, install_step_accessible, install_step_complete, install_step_index,
    install_step_index_badge, install_step_state_class, install_step_state_text,
    install_storage_ready, is_current_install_s3_request_confirmed, summary_or_pending,
};

#[component]
pub fn InstallWizardPage(
    bootstrap_status: BootstrapStatusResponse,
    initial_config: AdminSettingsConfig,
    #[props(default)] on_installed: EventHandler<InstallBootstrapResponse>,
) -> Element {
    let controller = use_install_wizard_controller(&initial_config, on_installed);

    let form = controller.form;
    let mut admin_email = controller.admin_email;
    let mut admin_password = controller.admin_password;
    let mut confirm_password = controller.confirm_password;
    let mut show_admin_password = controller.show_admin_password;
    let mut show_confirm_password = controller.show_confirm_password;
    let selected_favicon = controller.selected_favicon;
    let mut current_step = controller.current_step;
    let is_installing = controller.is_installing;
    let error_message = controller.error_message;
    let success_message = controller.success_message;
    let is_testing_s3 = controller.is_testing_s3;
    let last_tested_s3_request = controller.last_tested_s3_request;
    let s3_test_feedback = controller.s3_test_feedback;
    let s3_test_feedback_tone = controller.s3_test_feedback_tone;
    let storage_browser_open = controller.storage_browser_open;
    let storage_browser_loading = controller.storage_browser_loading;
    let storage_browser_error = controller.storage_browser_error;
    let storage_browser_current_path = controller.storage_browser_current_path;
    let storage_browser_parent_path = controller.storage_browser_parent_path;
    let storage_browser_directories = controller.storage_browser_directories;

    let handle_pick_favicon = {
        let controller = controller.clone();
        move |event: Event<FormData>| controller.pick_favicon(event)
    };

    let handle_install = {
        let controller = controller.clone();
        move || controller.install()
    };

    let handle_test_s3 = {
        let controller = controller.clone();
        move |_| controller.test_s3()
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
    let storage_summary = current_install_storage_summary(form);
    let mail_summary = current_install_mail_summary(form);
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
                                                onchange: {
                                                    let controller = controller.clone();
                                                    move |event| {
                                                        controller.set_storage_backend(StorageBackendKind::parse(&event.value()));
                                                    }
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
                                                        onclick: {
                                                            let controller = controller.clone();
                                                            move |_| controller.open_storage_browser()
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
                                        on_close: {
                                            let controller = controller.clone();
                                            move |_| controller.close_storage_browser()
                                        },
                                        div { class: "install-path-browser",
                                            div { class: "install-path-browser-head",
                                                code { class: "install-path-browser-current", "{storage_browser_current_path()}" }
                                                div { class: "install-path-browser-toolbar",
                                                    button {
                                                        class: "btn btn-ghost",
                                                        r#type: "button",
                                                        disabled: is_installing() || is_testing_s3() || storage_browser_loading() || storage_browser_parent_path().is_none(),
                                                        onclick: {
                                                            let controller = controller.clone();
                                                            move |_| controller.browse_storage_parent()
                                                        },
                                                        "上一级"
                                                    }
                                                    button {
                                                        class: "btn btn-primary",
                                                        r#type: "button",
                                                        disabled: is_installing() || is_testing_s3(),
                                                        onclick: {
                                                            let controller = controller.clone();
                                                            move |_| controller.select_current_storage_directory()
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
                                                            rsx! {
                                                                button {
                                                                    key: "{entry_path}",
                                                                    class: "install-path-browser-item",
                                                                    r#type: "button",
                                                                    disabled: is_installing() || is_testing_s3() || storage_browser_loading(),
                                                                    onclick: {
                                                                        let controller = controller.clone();
                                                                        move |_| controller.browse_storage_path(entry_path.clone())
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
