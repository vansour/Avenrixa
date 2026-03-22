mod page_controller;
mod summary;

use crate::pages::settings_page::render_general_fields_compact;
use crate::types::api::{AdminSettingsConfig, BootstrapStatusResponse, InstallBootstrapResponse};
use dioxus::prelude::*;

use page_controller::use_install_wizard_controller;
use summary::{
    INSTALL_WIZARD_STEPS, InstallWizardStep, current_install_mail_summary,
    current_install_storage_summary, install_admin_confirm_password_error,
    install_admin_email_error, install_admin_password_error, install_admin_ready,
    install_mail_ready, install_step_accessible, install_step_complete, install_step_index,
    install_step_index_badge, install_step_state_class, install_step_state_text,
    install_storage_ready, summary_or_pending,
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
    let mut current_step = controller.current_step;

    let handle_primary_action = move |_| {
        let step = current_step();
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
        "Dragonfly 外部缓存".to_string()
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
        .unwrap_or_else(|| "未配置 CACHE_URL".to_string());
    let cache_status_class = if bootstrap_status.cache_configured {
        "stat-pill stat-pill-active"
    } else {
        "stat-pill"
    };
    let runtime_error = bootstrap_status.runtime_error.clone().unwrap_or_default();
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
    let primary_action_label = format!(
        "下一步：{}",
        next_step
            .map(InstallWizardStep::label)
            .unwrap_or("确认安装")
    );
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
                                disabled: !install_step_accessible(
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
                                                }
                                                button {
                                                    class: "btn btn-ghost install-password-toggle",
                                                    r#type: "button",
                                                    onclick: move |_| show_admin_password.toggle(),
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
                                                }
                                                button {
                                                    class: "btn btn-ghost install-password-toggle",
                                                    r#type: "button",
                                                    onclick: move |_| show_confirm_password.toggle(),
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
                            {render_general_fields_compact(form, false)}
                        } else if current_step_value == InstallWizardStep::Storage {
                            div { class: "settings-stack",
                                div { class: "settings-subcard install-compact-subcard",
                                    h3 { "本地存储" }
                                    div { class: "settings-grid settings-grid-single",
                                        div { class: "settings-field settings-field-full",
                                            span { "本地存储路径（必填）" }
                                            input {
                                                class: "install-path-input",
                                                r#type: "text",
                                                value: "{(form.local_storage_path)()}",
                                                readonly: true,
                                                disabled: true,
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
                                        {render_install_review_row("站点名称", summary_or_pending((form.site_name)()))}
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
                                        {render_install_check_item("存储", "本地路径已配置", storage_ready)}
                                    }
                                }
                            }
                        }
                    }

                    div { class: "install-stage-actions",
                        div { class: "install-stage-actions-group",
                            button {
                                class: "btn btn-ghost install-back-button",
                                r#type: "button",
                                disabled: prev_step.is_none(),
                                onclick: move |_| {
                                    if let Some(step) = prev_step {
                                        current_step.set(step);
                                    }
                                },
                                "上一步"
                            }

                            button {
                                class: "btn btn-primary install-continue-button",
                                r#type: "button",
                                disabled: !current_step_ready || next_step.is_none(),
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
