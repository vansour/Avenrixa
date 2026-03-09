use crate::types::api::{
    AdminUserSummary, AuditLog, BackupResponse, ComponentStatus, HealthStatus, Setting, SystemStats,
};
use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use std::collections::HashMap;

use super::state::SettingsFormState;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    Account,
    General,
    Storage,
    Security,
    System,
    Maintenance,
    Users,
    Audit,
    Advanced,
}

pub const ADMIN_SETTINGS_SECTIONS: [SettingsSection; 9] = [
    SettingsSection::General,
    SettingsSection::Storage,
    SettingsSection::Security,
    SettingsSection::Account,
    SettingsSection::System,
    SettingsSection::Maintenance,
    SettingsSection::Users,
    SettingsSection::Audit,
    SettingsSection::Advanced,
];

pub const USER_SETTINGS_SECTIONS: [SettingsSection; 2] =
    [SettingsSection::Account, SettingsSection::Security];

impl SettingsSection {
    pub fn label(self) -> &'static str {
        match self {
            Self::Account => "账户",
            Self::General => "基础设置",
            Self::Storage => "存储设置",
            Self::Security => "账号安全",
            Self::System => "系统状态",
            Self::Maintenance => "维护工具",
            Self::Users => "用户与权限",
            Self::Audit => "审计日志",
            Self::Advanced => "高级设置",
        }
    }

    pub fn title(self) -> &'static str {
        self.label()
    }

    pub fn uses_global_settings_actions(self) -> bool {
        matches!(self, Self::General | Self::Storage)
    }
}

pub fn render_settings_fields(
    form: SettingsFormState,
    disabled: bool,
    section: SettingsSection,
) -> Element {
    match section {
        SettingsSection::General => render_general_fields(form, disabled),
        SettingsSection::Storage => render_storage_fields(form, disabled),
        _ => render_placeholder_section(section),
    }
}

#[component]
pub fn AccountSettingsSection(
    username: String,
    role: String,
    created_at: String,
    is_logging_out: bool,
    #[props(default)] on_logout: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "settings-stack",
            div { class: "settings-metric-grid",
                {render_metric_card("用户名", username)}
                {render_metric_card("角色", role_label(&role).to_string())}
                {render_metric_card("创建时间", created_at)}
            }

            div { class: "settings-action-grid",
                article { class: "settings-action-card settings-action-card-danger",
                    div { class: "settings-action-copy",
                        h3 { "退出登录" }
                    }
                    button {
                        class: "btn btn-danger",
                        disabled: is_logging_out,
                        onclick: move |event| on_logout.call(event),
                        if is_logging_out { "退出中..." } else { "退出登录" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn SecuritySettingsSection(
    current_password: Signal<String>,
    new_password: Signal<String>,
    confirm_password: Signal<String>,
    error_message: String,
    success_message: String,
    is_submitting: bool,
    #[props(default)] on_submit: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "settings-stack",
            if !error_message.is_empty() {
                div { class: "error-banner", "{error_message}" }
            }

            if !success_message.is_empty() {
                div { class: "settings-banner settings-banner-success", "{success_message}" }
            }

            div { class: "settings-grid settings-grid-single",
                label { class: "settings-field settings-field-full",
                    span { "当前密码" }
                    input {
                        r#type: "password",
                        value: "{current_password()}",
                        oninput: move |event| current_password.set(event.value()),
                        disabled: is_submitting,
                    }
                }

                label { class: "settings-field settings-field-full",
                    span { "新密码" }
                    input {
                        r#type: "password",
                        value: "{new_password()}",
                        oninput: move |event| new_password.set(event.value()),
                        disabled: is_submitting,
                    }
                }

                label { class: "settings-field settings-field-full",
                    span { "确认新密码" }
                    input {
                        r#type: "password",
                        value: "{confirm_password()}",
                        oninput: move |event| confirm_password.set(event.value()),
                        disabled: is_submitting,
                    }
                }
            }

            div { class: "settings-actions",
                button {
                    class: "btn btn-primary",
                    disabled: is_submitting,
                    onclick: move |event| on_submit.call(event),
                    if is_submitting { "修改中..." } else { "修改密码" }
                }
            }
        }
    }
}

#[component]
pub fn SystemStatusSection(
    health: Option<HealthStatus>,
    stats: Option<SystemStats>,
    error_message: String,
    is_loading: bool,
    #[props(default)] on_refresh: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "settings-stack",
            div { class: "settings-inline-actions",
                button {
                    class: "btn",
                    disabled: is_loading,
                    onclick: move |event| on_refresh.call(event),
                    if is_loading { "刷新中..." } else { "刷新状态" }
                }
            }

            if !error_message.is_empty() {
                div { class: "error-banner", "{error_message}" }
            }

            if let Some(health) = health.clone() {
                div { class: "settings-status-summary",
                    article { class: format!("settings-summary-card {}", status_surface_class(&health.status)),
                        p { class: "settings-summary-label", "系统状态" }
                        h3 { "{status_label(&health.status)}" }
                    }
                    article { class: "settings-summary-card",
                        p { class: "settings-summary-label", "运行版本" }
                        h3 { {health.version.unwrap_or_else(|| "未提供".to_string())} }
                    }
                }

                div { class: "settings-status-grid",
                    {render_component_status_card("数据库", &health.database)}
                    {render_component_status_card("缓存服务", &health.redis)}
                    {render_component_status_card("存储后端", &health.storage)}
                }

                if let Some(metrics) = health.metrics {
                    div { class: "settings-metric-grid",
                        {render_metric_card("健康检查图片数", metrics.images_count.to_string())}
                        {render_metric_card("健康检查用户数", metrics.users_count.to_string())}
                        {render_metric_card("估算存储用量", format_storage_mb(metrics.storage_used_mb))}
                    }
                }
            } else if is_loading {
                div { class: "settings-placeholder settings-placeholder-compact",
                    h3 { "正在加载系统状态" }
                }
            }

            if let Some(stats) = stats {
                div { class: "settings-metric-grid",
                    {render_metric_card("总用户数", stats.total_users.to_string())}
                    {render_metric_card("活跃图片数", stats.total_images.to_string())}
                    {render_metric_card("存储占用", format_storage_bytes(stats.total_storage))}
                    {render_metric_card("累计浏览量", stats.total_views.to_string())}
                    {render_metric_card("近 24 小时新增", stats.images_last_24h.to_string())}
                    {render_metric_card("近 7 天新增", stats.images_last_7d.to_string())}
                }
            }
        }
    }
}

#[component]
pub fn MaintenanceSettingsSection(
    error_message: String,
    success_message: String,
    last_backup: Option<BackupResponse>,
    last_deleted_cleanup_count: Option<usize>,
    last_expired_cleanup_count: Option<i64>,
    is_cleaning_deleted: bool,
    is_cleaning_expired: bool,
    is_backing_up: bool,
    #[props(default)] on_cleanup_deleted: EventHandler<MouseEvent>,
    #[props(default)] on_cleanup_expired: EventHandler<MouseEvent>,
    #[props(default)] on_backup: EventHandler<MouseEvent>,
) -> Element {
    let _ = (
        last_backup,
        last_deleted_cleanup_count,
        last_expired_cleanup_count,
    );

    rsx! {
        div { class: "settings-stack",
            if !error_message.is_empty() {
                div { class: "error-banner", "{error_message}" }
            }

            if !success_message.is_empty() {
                div { class: "settings-banner settings-banner-success", "{success_message}" }
            }

            div { class: "settings-action-grid",
                article { class: "settings-action-card settings-action-card-danger",
                    div { class: "settings-action-copy",
                        h3 { "清理已删除文件" }
                    }
                    button {
                        class: "btn btn-danger",
                        disabled: is_cleaning_deleted || is_cleaning_expired || is_backing_up,
                        onclick: move |event| on_cleanup_deleted.call(event),
                        if is_cleaning_deleted { "清理中..." } else { "执行清理" }
                    }
                }

                article { class: "settings-action-card",
                    div { class: "settings-action-copy",
                        h3 { "清理过期图片" }
                    }
                    button {
                        class: "btn",
                        disabled: is_cleaning_deleted || is_cleaning_expired || is_backing_up,
                        onclick: move |event| on_cleanup_expired.call(event),
                        if is_cleaning_expired { "处理中..." } else { "处理过期图片" }
                    }
                }

                article { class: "settings-action-card settings-action-card-accent",
                    div { class: "settings-action-copy",
                        h3 { "数据库备份" }
                    }
                    button {
                        class: "btn btn-primary",
                        disabled: is_cleaning_deleted || is_cleaning_expired || is_backing_up,
                        onclick: move |event| on_backup.call(event),
                        if is_backing_up { "备份中..." } else { "生成备份" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn UsersSettingsSection(
    users: Vec<AdminUserSummary>,
    role_drafts: Signal<HashMap<String, String>>,
    error_message: String,
    success_message: String,
    is_loading: bool,
    updating_user_id: Option<String>,
    #[props(default)] on_refresh: EventHandler<MouseEvent>,
    #[props(default)] on_save_role: EventHandler<String>,
) -> Element {
    let is_updating_any = updating_user_id.is_some();

    rsx! {
        div { class: "settings-stack",
            div { class: "settings-list-toolbar",
                div { class: "settings-toolbar-meta",
                    span { class: "stat-pill", "账户 {users.len()} 个" }
                }
                div { class: "settings-inline-actions",
                    button {
                        class: "btn",
                        disabled: is_loading || is_updating_any,
                        onclick: move |event| on_refresh.call(event),
                        if is_loading { "刷新中..." } else { "刷新列表" }
                    }
                }
            }

            if !error_message.is_empty() {
                div { class: "error-banner", "{error_message}" }
            }

            if !success_message.is_empty() {
                div { class: "settings-banner settings-banner-success", "{success_message}" }
            }

            if is_loading && users.is_empty() {
                div { class: "settings-placeholder settings-placeholder-compact",
                    h3 { "正在加载用户列表" }
                }
            } else if users.is_empty() {
                div { class: "settings-placeholder settings-placeholder-compact",
                    h3 { "暂时没有可展示的用户" }
                }
            } else {
                div { class: "settings-entity-list",
                    {users.into_iter().map(|user| {
                        let user_id_for_select = user.id.clone();
                        let user_id_for_save = user.id.clone();
                        let current_role = role_drafts()
                            .get(&user.id)
                            .cloned()
                            .unwrap_or_else(|| user.role.clone());
                        let is_row_updating = updating_user_id.as_deref() == Some(user.id.as_str());
                        rsx! {
                            article { class: "settings-entity-card",
                                div { class: "settings-entity-main",
                                    div { class: "settings-entity-copy",
                                        div { class: "settings-entity-title",
                                            h3 { "{user.username}" }
                                            span {
                                                class: format!(
                                                    "settings-role-badge {}",
                                                    role_surface_class(&user.role)
                                                ),
                                                {role_label(&user.role)}
                                            }
                                        }
                                        p { class: "settings-entity-meta",
                                            "用户 ID {short_identifier(&user.id)} · 创建于 {format_timestamp(user.created_at)}"
                                        }
                                    }

                                    div { class: "settings-entity-controls",
                                        label { class: "settings-field settings-inline-field",
                                            span { "角色" }
                                            select {
                                                value: "{current_role}",
                                                disabled: is_loading || is_updating_any,
                                                onchange: move |event| {
                                                    let mut drafts = role_drafts();
                                                    drafts.insert(user_id_for_select.clone(), event.value());
                                                    role_drafts.set(drafts);
                                                },
                                                option { value: "admin", "admin" }
                                                option { value: "user", "user" }
                                            }
                                        }
                                        button {
                                            class: "btn btn-primary",
                                            disabled: is_loading || is_updating_any,
                                            onclick: move |_| on_save_role.call(user_id_for_save.clone()),
                                            if is_row_updating { "保存中..." } else { "保存角色" }
                                        }
                                    }
                                }
                            }
                        }
                    })}
                }
            }
        }
    }
}

#[component]
pub fn AuditSettingsSection(
    logs: Vec<AuditLog>,
    total: i64,
    current_page: i32,
    page_size: i32,
    has_next: bool,
    error_message: String,
    is_loading: bool,
    #[props(default)] on_refresh: EventHandler<MouseEvent>,
    #[props(default)] on_prev_page: EventHandler<MouseEvent>,
    #[props(default)] on_next_page: EventHandler<MouseEvent>,
    #[props(default)] on_page_size_change: EventHandler<String>,
) -> Element {
    let (start, end) = page_window(current_page, page_size, total);

    rsx! {
        div { class: "settings-stack",
            div { class: "settings-list-toolbar",
                div { class: "settings-toolbar-meta",
                    span { class: "stat-pill", "日志总数 {total}" }
                    span { class: "stat-pill", "当前第 {current_page} 页" }
                    if total > 0 {
                        span { class: "stat-pill stat-pill-active", "显示 {start}-{end}" }
                    }
                }
                div { class: "settings-list-actions",
                    label { class: "page-size-control",
                        span { "每页" }
                        select {
                            class: "page-size-select",
                            value: "{page_size}",
                            disabled: is_loading,
                            onchange: move |event| on_page_size_change.call(event.value()),
                            option { value: "20", "20" }
                            option { value: "50", "50" }
                            option { value: "100", "100" }
                        }
                        span { "条" }
                    }
                    button {
                        class: "btn",
                        disabled: is_loading,
                        onclick: move |event| on_refresh.call(event),
                        if is_loading { "刷新中..." } else { "刷新日志" }
                    }
                    div { class: "page-actions",
                        button {
                            class: "btn",
                            disabled: is_loading || current_page <= 1,
                            onclick: move |event| on_prev_page.call(event),
                            "上一页"
                        }
                        button {
                            class: "btn",
                            disabled: is_loading || !has_next,
                            onclick: move |event| on_next_page.call(event),
                            "下一页"
                        }
                    }
                }
            }

            if !error_message.is_empty() {
                div { class: "error-banner", "{error_message}" }
            }

            if is_loading && logs.is_empty() {
                div { class: "settings-placeholder settings-placeholder-compact",
                    h3 { "正在加载审计日志" }
                }
            } else if logs.is_empty() {
                div { class: "settings-placeholder settings-placeholder-compact",
                    h3 { "暂无审计记录" }
                }
            } else {
                div { class: "settings-log-list",
                    for log in logs {
                        article { class: "settings-log-card",
                            div { class: "settings-log-head",
                                div { class: "settings-log-title",
                                    h3 { "{humanize_action(&log.action)}" }
                                    p { class: "settings-log-meta",
                                        "{format_timestamp(log.created_at)} · 目标 {log.target_type}"
                                    }
                                }
                                span { class: "settings-log-tag", "{log.action}" }
                            }

                            div { class: "settings-log-meta-grid",
                                span { class: "settings-log-chip", "用户 {optional_short_id(log.user_id.as_deref())}" }
                                span { class: "settings-log-chip", "目标ID {optional_short_id(log.target_id.as_deref())}" }
                                span { class: "settings-log-chip", {"IP ".to_string() + &log.ip_address.clone().unwrap_or_else(|| "未知".to_string())} }
                            }

                            if let Some(details) = &log.details {
                                pre { class: "settings-code-block", "{format_json_details(details)}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AdvancedSettingsSection(
    settings: Vec<Setting>,
    setting_drafts: Signal<HashMap<String, String>>,
    error_message: String,
    success_message: String,
    is_loading: bool,
    saving_key: Option<String>,
    #[props(default)] on_refresh: EventHandler<MouseEvent>,
    #[props(default)] on_save_setting: EventHandler<String>,
) -> Element {
    let is_saving_any = saving_key.is_some();

    rsx! {
        div { class: "settings-stack",
            div { class: "settings-list-toolbar",
                div { class: "settings-toolbar-meta",
                    span { class: "stat-pill", "键值 {settings.len()} 项" }
                }
                div { class: "settings-inline-actions",
                    button {
                        class: "btn",
                        disabled: is_loading || is_saving_any,
                        onclick: move |event| on_refresh.call(event),
                        if is_loading { "刷新中..." } else { "刷新键值" }
                    }
                }
            }

            if !error_message.is_empty() {
                div { class: "error-banner", "{error_message}" }
            }

            if !success_message.is_empty() {
                div { class: "settings-banner settings-banner-success", "{success_message}" }
            }

            if is_loading && settings.is_empty() {
                div { class: "settings-placeholder settings-placeholder-compact",
                    h3 { "正在加载原始设置" }
                }
            } else if settings.is_empty() {
                div { class: "settings-placeholder settings-placeholder-compact",
                    h3 { "暂时没有原始键值" }
                }
            } else {
                div { class: "settings-kv-list",
                    {settings.into_iter().map(|setting| {
                        let setting_key_for_input = setting.key.clone();
                        let setting_key_for_save = setting.key.clone();
                        let draft_value = setting_drafts()
                            .get(&setting.key)
                            .cloned()
                            .unwrap_or_else(|| setting.value.clone());
                        let is_row_saving = saving_key.as_deref() == Some(setting.key.as_str());
                        let is_readonly = !setting.editable;
                        rsx! {
                            article { class: "settings-kv-card",
                                div { class: "settings-kv-head",
                                    div { class: "settings-kv-copy",
                                        h3 { class: "settings-kv-key", "{setting.key}" }
                                        div { class: "settings-kv-badges",
                                            if is_readonly {
                                                span { class: "settings-kv-badge", "只读" }
                                            }
                                            if setting.masked {
                                                span { class: "settings-kv-badge is-warning", "已脱敏" }
                                            }
                                            if setting.requires_confirmation {
                                                span { class: "settings-kv-badge is-warning", "需确认" }
                                            }
                                        }
                                    }
                                    button {
                                        class: "btn btn-primary",
                                        disabled: is_loading || is_saving_any || is_readonly,
                                        onclick: move |_| on_save_setting.call(setting_key_for_save.clone()),
                                        if is_readonly {
                                            "受保护"
                                        } else if is_row_saving {
                                            "保存中..."
                                        } else {
                                            "保存键值"
                                        }
                                    }
                                }

                                textarea {
                                    class: "settings-kv-input",
                                    value: "{draft_value}",
                                    rows: "{textarea_rows(&draft_value)}",
                                    disabled: is_loading || is_saving_any || is_readonly,
                                    oninput: move |event| {
                                        let mut drafts = setting_drafts();
                                        drafts.insert(setting_key_for_input.clone(), event.value());
                                        setting_drafts.set(drafts);
                                    },
                                }
                            }
                        }
                    })}
                }
            }
        }
    }
}

fn render_general_fields(form: SettingsFormState, disabled: bool) -> Element {
    let mut site_name = form.site_name;

    rsx! {
        div { class: "settings-stack",
            div { class: "settings-grid settings-grid-single",
                label { class: "settings-field settings-field-full",
                    span { "网站名称" }
                    input {
                        r#type: "text",
                        value: "{site_name()}",
                        oninput: move |event| site_name.set(event.value()),
                        disabled,
                    }
                }
            }
        }
    }
}

fn render_storage_fields(form: SettingsFormState, disabled: bool) -> Element {
    let mut storage_backend = form.storage_backend;
    let mut local_storage_path = form.local_storage_path;
    let show_s3_fields = form.is_s3_backend();

    rsx! {
        div { class: "settings-stack",
            div { class: "settings-grid",
                label { class: "settings-field",
                    span { "存储后端" }
                    select {
                        value: "{storage_backend()}",
                        onchange: move |event| storage_backend.set(event.value()),
                        disabled,
                        option { value: "local", "local" }
                        option { value: "s3", "s3" }
                    }
                }

                label { class: "settings-field settings-field-full",
                    span { "本地存储路径" }
                    input {
                        r#type: "text",
                        value: "{local_storage_path()}",
                        oninput: move |event| local_storage_path.set(event.value()),
                        disabled,
                    }
                }

                if show_s3_fields {
                    {render_s3_fields(form, disabled)}
                }
            }
        }
    }
}

fn render_s3_fields(form: SettingsFormState, disabled: bool) -> Element {
    let mut s3_endpoint = form.s3_endpoint;
    let mut s3_region = form.s3_region;
    let mut s3_bucket = form.s3_bucket;
    let mut s3_prefix = form.s3_prefix;
    let mut s3_access_key = form.s3_access_key;
    let mut s3_secret_key = form.s3_secret_key;
    let s3_secret_key_set = form.s3_secret_key_set;
    let mut s3_force_path_style = form.s3_force_path_style;

    rsx! {
        label { class: "settings-field",
            span { "S3 Endpoint" }
            input {
                r#type: "text",
                value: "{s3_endpoint()}",
                oninput: move |event| s3_endpoint.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-field",
            span { "S3 Region" }
            input {
                r#type: "text",
                value: "{s3_region()}",
                oninput: move |event| s3_region.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-field",
            span { "S3 Bucket" }
            input {
                r#type: "text",
                value: "{s3_bucket()}",
                oninput: move |event| s3_bucket.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-field",
            span { "S3 Prefix（可选）" }
            input {
                r#type: "text",
                value: "{s3_prefix()}",
                oninput: move |event| s3_prefix.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-field",
            span { "S3 Access Key" }
            input {
                r#type: "text",
                value: "{s3_access_key()}",
                oninput: move |event| s3_access_key.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-field",
            span {
                if s3_secret_key_set() {
                    "S3 Secret Key（留空表示不修改）"
                } else {
                    "S3 Secret Key"
                }
            }
            input {
                r#type: "password",
                value: "{s3_secret_key()}",
                oninput: move |event| s3_secret_key.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-check",
            input {
                r#type: "checkbox",
                checked: s3_force_path_style(),
                onchange: move |event| s3_force_path_style.set(event.checked()),
                disabled,
            }
            span { "S3 Force Path Style（MinIO 通常需要开启）" }
        }
    }
}

fn render_placeholder_section(section: SettingsSection) -> Element {
    rsx! {
        div { class: "settings-placeholder settings-placeholder-compact",
            h3 { "{section.title()}" }
        }
    }
}

fn render_component_status_card(title: &str, status: &ComponentStatus) -> Element {
    rsx! {
        article { class: format!("settings-status-card {}", status_surface_class(&status.status)),
            p { class: "settings-summary-label", "{title}" }
            h3 { "{status_label(&status.status)}" }
            if let Some(message) = &status.message {
                p { class: "settings-status-message", "{message}" }
            } else {
                p { class: "settings-status-message", "运行正常" }
            }
        }
    }
}

fn render_metric_card(title: &str, value: String) -> Element {
    rsx! {
        article { class: "settings-metric-card",
            p { class: "settings-summary-label", "{title}" }
            h3 { "{value}" }
        }
    }
}

fn status_label(status: &str) -> &'static str {
    if status.eq_ignore_ascii_case("healthy") {
        "健康"
    } else {
        "异常"
    }
}

fn status_surface_class(status: &str) -> &'static str {
    if status.eq_ignore_ascii_case("healthy") {
        "is-healthy"
    } else {
        "is-unhealthy"
    }
}

fn role_label(role: &str) -> &'static str {
    if role.eq_ignore_ascii_case("admin") {
        "管理员"
    } else {
        "普通用户"
    }
}

fn role_surface_class(role: &str) -> &'static str {
    if role.eq_ignore_ascii_case("admin") {
        "is-admin"
    } else {
        "is-user"
    }
}

fn humanize_action(action: &str) -> String {
    action
        .split('_')
        .filter(|segment| !segment.trim().is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_timestamp(timestamp: DateTime<Utc>) -> String {
    timestamp.format("%Y-%m-%d %H:%M UTC").to_string()
}

fn format_storage_bytes(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn format_storage_mb(storage_used_mb: Option<f64>) -> String {
    storage_used_mb
        .map(|value| format!("{value:.2} MB"))
        .unwrap_or_else(|| "未知".to_string())
}

fn short_identifier(value: &str) -> String {
    value.chars().take(8).collect()
}

fn optional_short_id(value: Option<&str>) -> String {
    value
        .map(short_identifier)
        .unwrap_or_else(|| "未知".to_string())
}

fn format_json_details(details: &serde_json::Value) -> String {
    serde_json::to_string_pretty(details)
        .or_else(|_| serde_json::to_string(details))
        .unwrap_or_else(|_| "无法序列化详情".to_string())
}

fn textarea_rows(value: &str) -> usize {
    let lines = value.lines().count().max(1);
    lines.clamp(3, 8)
}

fn page_window(page: i32, page_size: i32, total: i64) -> (i64, i64) {
    if total <= 0 {
        return (0, 0);
    }

    let page = page.max(1) as i64;
    let page_size = page_size.max(1) as i64;
    let start = (page - 1) * page_size + 1;
    let end = (page * page_size).min(total);
    (start, end)
}
