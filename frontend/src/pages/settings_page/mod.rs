mod controllers;
mod page_controller;
mod state;
mod summary;
mod view;

use crate::app_context::use_navigation_store;
use crate::store::SettingsAnchor;
use dioxus::prelude::*;

use controllers::{
    AccountSectionController, MaintenanceSectionController, SecuritySectionController,
    SystemSectionController, UsersSectionController,
};
use page_controller::{S3TestFeedbackTone, use_settings_page_controller};
pub(super) use page_controller::{handle_settings_auth_error, settings_auth_expired_message};
use summary::{
    count_config_changes, current_mail_summary, current_storage_summary,
    is_current_s3_request_confirmed, requires_s3_test_confirmation, resolved_settings_section,
    settings_anchor_for_section,
};
use view::{
    ADMIN_SETTINGS_SECTIONS, SettingsSection, USER_SETTINGS_SECTIONS, render_settings_fields,
};

pub use state::infer_s3_provider_preset;
pub use state::{S3ProviderPreset, SettingsFormState, default_mail_link_base_url};
pub use view::{
    render_general_fields, render_general_fields_compact, render_s3_fields,
    render_s3_fields_compact, render_storage_fields, render_storage_fields_compact,
};

#[component]
pub fn SettingsPage(
    is_admin: bool,
    #[props(default)] requested_section: Option<SettingsAnchor>,
    #[props(default)] on_site_name_updated: EventHandler<String>,
) -> Element {
    let navigation_store = use_navigation_store();
    let controller = use_settings_page_controller(is_admin, on_site_name_updated);

    let settings_sections: &[SettingsSection] = if is_admin {
        &ADMIN_SETTINGS_SECTIONS
    } else {
        &USER_SETTINGS_SECTIONS
    };
    let current_section = resolved_settings_section(is_admin, requested_section);
    let requires_s3_test = controller
        .loaded_config()
        .as_ref()
        .is_some_and(|config| requires_s3_test_confirmation(controller.form, config));
    let s3_test_confirmed = !requires_s3_test
        || is_current_s3_request_confirmed(controller.form, controller.last_tested_s3_request());
    let is_form_disabled =
        controller.is_loading() || controller.is_saving() || controller.is_testing_s3();
    let save_disabled = is_form_disabled || (requires_s3_test && !s3_test_confirmed);
    let has_restart_notice = controller
        .loaded_config()
        .as_ref()
        .is_some_and(|config| config.restart_required);
    let changed_fields = controller
        .loaded_config()
        .as_ref()
        .map(|config| count_config_changes(controller.form, config))
        .unwrap_or(0);
    let has_unsaved_changes = changed_fields > 0;
    let storage_summary = current_storage_summary(controller.form);
    let mail_summary = current_mail_summary(controller.form);
    let controller_for_test = controller.clone();
    let controller_for_refresh = controller.clone();
    let controller_for_save = controller.clone();
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
                                onclick: {
                                    let navigation_store = navigation_store.clone();
                                    move |_| {
                                        navigation_store.open_settings(settings_anchor_for_section(section));
                                    }
                                },
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

                        if !controller.error_message().is_empty() && current_section.uses_global_settings_actions() {
                            div { class: "error-banner", "{controller.error_message()}" }
                        }

                        {
                            match current_section {
                                SettingsSection::Account => rsx! { AccountSectionController {} },
                                SettingsSection::Security => rsx! { SecuritySectionController {} },
                                SettingsSection::System => rsx! { SystemSectionController {} },
                                SettingsSection::Maintenance => rsx! { MaintenanceSectionController {} },
                                SettingsSection::Users => rsx! { UsersSectionController {} },
                                _ => render_settings_fields(controller.form, is_form_disabled, current_section),
                            }
                        }

                        if current_section == SettingsSection::Storage && controller.form.is_s3_backend() {
                            div { class: "settings-stack",
                                div { class: "settings-actions",
                                    button {
                                        class: "btn btn-primary",
                                        onclick: move |_| controller_for_test.test_s3(),
                                        disabled: is_form_disabled,
                                        if controller.is_testing_s3() {
                                            "测试中..."
                                        } else {
                                            "测试 S3 连通性"
                                        }
                                    }
                                }

                                if !controller.s3_test_feedback().is_empty() {
                                    div {
                                        class: match controller.s3_test_feedback_tone() {
                                            S3TestFeedbackTone::Success => "settings-banner settings-banner-success",
                                            S3TestFeedbackTone::Error => "error-banner",
                                            S3TestFeedbackTone::Neutral => "settings-banner settings-banner-warning",
                                        },
                                        "{controller.s3_test_feedback()}"
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
                                    onclick: move |_| controller_for_refresh.refresh(),
                                    disabled: is_form_disabled,
                                    if controller.is_loading() { "加载中..." } else { "刷新" }
                                }
                                button {
                                    class: "btn btn-primary",
                                    onclick: move |_| controller_for_save.save(),
                                    disabled: save_disabled,
                                    if controller.is_saving() { "保存中..." } else { "保存设置" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
