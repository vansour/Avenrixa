use crate::app_context::{use_auth_store, use_settings_service, use_toast_store};
use crate::components::Modal;
use crate::services::SettingsService;
use crate::types::api::{
    AdminUserSummary, AuditLog, BackupFileSummary, BackupResponse, BackupRestoreStatusResponse,
    BackupSemantics, ComponentStatus, HealthStatus, Setting, StorageDirectoryEntry, SystemStats,
};
use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use std::collections::HashMap;

use super::state::SettingsFormState;
use super::{handle_settings_auth_error, settings_auth_expired_message};

const DEFAULT_SETTINGS_STORAGE_BROWSER_PATH: &str = "/data/images";

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

    pub fn description(self) -> &'static str {
        match self {
            Self::Account => "查看当前登录账户信息，并管理当前会话。",
            Self::General => "维护站点名称、邮件发送和验证链接等基础配置。",
            Self::Storage => "配置图片写入位置，并决定使用本地目录还是对象存储。",
            Self::Security => "修改当前账户密码，收紧登录安全。",
            Self::System => "查看健康检查、容量和整体运行状态。",
            Self::Maintenance => "执行清理与备份等高风险维护操作。",
            Self::Users => "查看账户列表，并调整用户权限。",
            Self::Audit => "追踪关键管理操作与系统事件。",
            Self::Advanced => "处理底层键值配置，仅在明确知道影响时修改。",
        }
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
    email: String,
    role: String,
    created_at: String,
    is_logging_out: bool,
    #[props(default)] on_logout: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "settings-stack",
            div { class: "settings-metric-grid",
                {render_metric_card("邮箱", email)}
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
    helper_text: String,
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

            if !helper_text.is_empty() {
                p { class: "settings-section-copy", "{helper_text}" }
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
                    {render_component_status_card("缓存服务", &health.cache)}
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
    backup_files: Vec<(BackupFileSummary, String)>,
    restore_status: Option<BackupRestoreStatusResponse>,
    last_expired_cleanup_count: Option<i64>,
    is_cleaning_expired: bool,
    is_backing_up: bool,
    deleting_backup_filename: Option<String>,
    processing_restore_filename: Option<String>,
    is_loading_backups: bool,
    is_loading_restore_status: bool,
    #[props(default)] on_cleanup_expired: EventHandler<MouseEvent>,
    #[props(default)] on_backup: EventHandler<MouseEvent>,
    #[props(default)] on_refresh_backups: EventHandler<MouseEvent>,
    #[props(default)] on_refresh_restore_status: EventHandler<MouseEvent>,
    #[props(default)] on_delete_backup: EventHandler<String>,
    #[props(default)] on_restore_backup: EventHandler<BackupFileSummary>,
) -> Element {
    let last_backup_name = last_backup
        .as_ref()
        .map(|backup| backup.filename.clone())
        .unwrap_or_else(|| "暂无备份".to_string());
    let last_backup_time = last_backup
        .as_ref()
        .map(|backup| format_timestamp(backup.created_at))
        .unwrap_or_else(|| "未生成".to_string());
    let expired_cleanup_summary = last_expired_cleanup_count
        .map(|count| format!("{} 张图片", count))
        .unwrap_or_else(|| "未执行".to_string());
    let pending_restore = restore_status
        .as_ref()
        .and_then(|status| status.pending.clone());
    let last_restore_result = restore_status
        .as_ref()
        .and_then(|status| status.last_result.clone());
    let pending_restore_filename = pending_restore.as_ref().map(|item| item.filename.clone());
    let pending_restore_summary = pending_restore
        .as_ref()
        .map(|item| item.filename.clone())
        .unwrap_or_else(|| "无".to_string());
    let pending_restore_time = pending_restore
        .as_ref()
        .map(|item| format_timestamp(item.scheduled_at))
        .unwrap_or_else(|| "未计划".to_string());
    let last_restore_status = last_restore_result
        .as_ref()
        .map(|item| restore_status_label(&item.status).to_string())
        .unwrap_or_else(|| "暂无记录".to_string());
    let last_restore_time = last_restore_result
        .as_ref()
        .map(|item| format_timestamp(item.finished_at))
        .unwrap_or_else(|| "未执行".to_string());
    let maintenance_busy = is_cleaning_expired
        || is_backing_up
        || deleting_backup_filename.is_some()
        || processing_restore_filename.is_some();
    let has_pending_restore = pending_restore.is_some();
    let pending_restore_count = if has_pending_restore { 1 } else { 0 };

    rsx! {
        div { class: "settings-stack",
            if !error_message.is_empty() {
                div { class: "error-banner", "{error_message}" }
            }

            if !success_message.is_empty() {
                div { class: "settings-banner settings-banner-success", "{success_message}" }
            }

            div { class: "settings-banner settings-banner-neutral",
                "维护工具已启用分级确认：过期图片永久删除和数据库恢复都属于高风险操作，需要输入确认词；数据库备份可直接执行。"
            }

            div { class: "settings-metric-grid",
                {render_metric_card("最近备份文件", last_backup_name)}
                {render_metric_card("最近备份时间", last_backup_time)}
                {render_metric_card("最近过期删除", expired_cleanup_summary)}
            }

            div { class: "settings-action-grid",
                article { class: "settings-action-card",
                    div { class: "settings-action-copy",
                        div { class: "settings-action-meta",
                            span { class: "settings-risk-badge is-danger", "Danger" }
                        }
                        h3 { "永久删除过期图片" }
                        p { class: "settings-action-note", "批量删除所有已过期图片，并同步移除文件与数据库记录。" }
                    }
                    button {
                        class: "btn btn-danger",
                        disabled: maintenance_busy,
                        onclick: move |event| on_cleanup_expired.call(event),
                        if is_cleaning_expired { "删除中..." } else { "执行删除" }
                    }
                }

                article { class: "settings-action-card settings-action-card-accent",
                    div { class: "settings-action-copy",
                        div { class: "settings-action-meta",
                            span { class: "settings-risk-badge is-safe", "Safe" }
                        }
                        h3 { "数据库备份" }
                        p { class: "settings-action-note", "生成当前数据库级备份；SQLite 会导出数据库快照，MySQL / MariaDB 与 PostgreSQL 会导出逻辑备份。" }
                    }
                    button {
                        class: "btn btn-primary",
                        disabled: maintenance_busy,
                        onclick: move |event| on_backup.call(event),
                        if is_backing_up { "备份中..." } else { "生成备份" }
                    }
                }
            }

            div { class: "settings-subcard",
                h3 { "数据库恢复状态" }
                p { class: "settings-section-copy",
                    "当前恢复是冷恢复流程：写入计划后会在下一次启动前执行。按 1.0 范围，当前页面内的 SQLite 数据库快照恢复仅作为 Experimental 能力保留；PostgreSQL 是默认 GA 主路径，但恢复统一走运维脚本。"
                }

                div { class: "settings-list-toolbar",
                    div { class: "settings-toolbar-meta",
                        span { class: "stat-pill", "待执行计划 {pending_restore_count}" }
                        if is_loading_restore_status {
                            span { class: "stat-pill stat-pill-warning", "状态刷新中" }
                        }
                    }
                    div { class: "settings-inline-actions",
                        button {
                            class: "btn",
                            disabled: maintenance_busy || is_loading_restore_status,
                            onclick: move |event| on_refresh_restore_status.call(event),
                            if is_loading_restore_status { "刷新中..." } else { "刷新恢复状态" }
                        }
                    }
                }

                if is_loading_restore_status && restore_status.is_none() {
                    div { class: "settings-placeholder settings-placeholder-compact",
                        h3 { "正在加载恢复状态" }
                    }
                } else {
                    div { class: "settings-metric-grid",
                        {render_metric_card("待执行恢复", pending_restore_summary)}
                        {render_metric_card("计划写入时间", pending_restore_time)}
                        {render_metric_card("最近恢复结果", last_restore_status)}
                        {render_metric_card("最近结果时间", last_restore_time)}
                    }

                    if let Some(pending) = pending_restore.clone() {
                        div { class: "settings-banner settings-banner-warning",
                            "检测到待执行的数据库恢复计划。请立即重启服务；真正的数据库替换或导入会在下一次启动前完成。"
                        }
                        article { class: "settings-entity-card",
                            div { class: "settings-entity-main",
                                div { class: "settings-entity-copy",
                                    div { class: "settings-entity-title",
                                        h3 { "{pending.filename}" }
                                        span { class: "settings-kv-badge is-warning", "{restore_database_kind_label(&pending.database_kind)} · 待执行" }
                                    }
                                    p { class: "settings-entity-meta",
                                        "计划写入于 {format_timestamp(pending.scheduled_at)} · 备份创建于 {format_timestamp(pending.backup_created_at)} · {format_storage_bytes_u64(pending.backup_size_bytes)}"
                                    }
                                    p { class: "settings-action-note",
                                        "申请人 {pending.requested_by_email}。恢复完成后，当前所有登录会话都会失效。"
                                    }
                                }
                            }
                        }
                    } else {
                        div { class: "settings-banner settings-banner-neutral",
                            "当前没有待执行的数据库恢复计划。当前页面内只有 SQLite 数据库快照可写入恢复计划，且这条能力在 1.0 范围内按 Experimental 保留；其他备份类型仅支持下载或运维侧恢复。"
                        }
                    }

                    if let Some(result) = last_restore_result.clone() {
                        article { class: "settings-entity-card",
                            div { class: "settings-entity-main",
                                div { class: "settings-entity-copy",
                                    div { class: "settings-entity-title",
                                        h3 { "最近一次恢复结果" }
                                        span {
                                            class: format!(
                                                "settings-kv-badge {}",
                                                restore_status_surface_class(&result.status)
                                            ),
                                            "{restore_status_label(&result.status)}"
                                        }
                                    }
                                    p { class: "settings-entity-meta",
                                        "{restore_database_kind_label(&result.database_kind)} 备份 {result.filename} · 完成于 {format_timestamp(result.finished_at)}"
                                    }
                                    p { class: "settings-action-note", "{result.message}" }
                                    if let Some(rollback_filename) = result.rollback_filename.clone() {
                                        p { class: "settings-entity-meta", "回滚快照 {rollback_filename}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "settings-subcard",
                h3 { "备份文件" }
                p { class: "settings-section-copy",
                    "这里展示后台生成的数据库级备份。SQLite 数据库快照仍可从当前页面写入恢复计划，但这条能力在 1.0 范围内按 Experimental 保留；MySQL / MariaDB 逻辑导出与 PostgreSQL 导出当前仅支持下载或运维侧恢复。"
                }

                div { class: "settings-list-toolbar",
                    div { class: "settings-toolbar-meta",
                        span { class: "stat-pill", "可下载备份 {backup_files.len()} 个" }
                        if is_loading_backups {
                            span { class: "stat-pill stat-pill-warning", "列表刷新中" }
                        }
                    }
                    div { class: "settings-inline-actions",
                        button {
                            class: "btn",
                            disabled: is_loading_backups || maintenance_busy,
                            onclick: move |event| on_refresh_backups.call(event),
                            if is_loading_backups { "刷新中..." } else { "刷新备份列表" }
                        }
                    }
                }

                if is_loading_backups && backup_files.is_empty() {
                    div { class: "settings-placeholder settings-placeholder-compact",
                        h3 { "正在加载备份列表" }
                    }
                } else if backup_files.is_empty() {
                    div { class: "settings-placeholder settings-placeholder-compact",
                        h3 { "暂时没有可下载的备份" }
                    }
                } else {
                    div { class: "settings-entity-list",
                        {backup_files.into_iter().map(|(backup, download_url)| {
                            let filename_for_download = backup.filename.clone();
                            let filename_for_delete = backup.filename.clone();
                            let backup_for_restore = backup.clone();
                            let kind_label = backup_kind_label(&backup.semantics);
                            let backup_meta = format!(
                                "{} · {}",
                                format_timestamp(backup.created_at),
                                format_storage_bytes_u64(backup.size_bytes)
                            );
                            let is_row_deleting = deleting_backup_filename
                                .as_deref()
                                .is_some_and(|value| value == backup.filename.as_str());
                            let is_row_restoring = processing_restore_filename
                                .as_deref()
                                .is_some_and(|value| value == backup.filename.as_str());
                            let is_pending_target = pending_restore_filename
                                .as_deref()
                                .is_some_and(|value| value == backup.filename.as_str());
                            let supports_restore = backup_supports_restore(&backup.semantics);
                            let is_experimental_page_restore =
                                backup.semantics.backup_kind == "sqlite-database-snapshot";
                            rsx! {
                                article { class: "settings-entity-card",
                                    div { class: "settings-entity-main",
                                        div { class: "settings-entity-copy",
                                            div { class: "settings-entity-title",
                                                h3 { "{backup.filename}" }
                                                div { class: "settings-kv-badges",
                                                    span { class: "settings-kv-badge", "{kind_label}" }
                                                    if is_experimental_page_restore {
                                                        span { class: "settings-kv-badge is-warning", "Experimental" }
                                                    }
                                                }
                                            }
                                            p { class: "settings-entity-meta", "{backup_meta}" }
                                            if is_experimental_page_restore {
                                                p { class: "settings-action-note",
                                                    "当前页面内的 SQLite 恢复在 1.0 范围内按 Experimental 保留，适合受控环境验证，不属于默认 GA 发布承诺。"
                                                }
                                            } else if !supports_restore {
                                                p { class: "settings-action-note", "当前这类备份仅支持下载或运维侧恢复，不支持当前页面恢复。" }
                                            }
                                        }

                                        div { class: "settings-entity-controls",
                                            a {
                                                class: "btn btn-primary",
                                                href: "{download_url}",
                                                download: "{filename_for_download}",
                                                "下载备份"
                                            }
                                            button {
                                                class: "btn btn-danger",
                                                disabled: !supports_restore || maintenance_busy || is_loading_restore_status || has_pending_restore,
                                                onclick: move |_| on_restore_backup.call(backup_for_restore.clone()),
                                                if is_row_restoring {
                                                    "处理中..."
                                                } else if is_pending_target {
                                                    "已计划恢复"
                                                } else if !supports_restore {
                                                    "不支持页面恢复"
                                                } else {
                                                    "恢复到此备份"
                                                }
                                            }
                                            button {
                                                class: "btn btn-danger",
                                                disabled: maintenance_busy || is_loading_backups || has_pending_restore,
                                                onclick: move |_| on_delete_backup.call(filename_for_delete.clone()),
                                                if is_row_deleting { "删除中..." } else { "删除备份" }
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

            div { class: "settings-banner settings-banner-neutral",
                "用户角色变更属于权限操作：普通提升会要求二次确认，管理员降级会升级为高风险确认。"
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
                                            h3 { "{user.email}" }
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
                        {render_audit_log_card(log)}
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

            div { class: "settings-banner settings-banner-neutral",
                "原始键值会直接写入底层 settings 表。标记为“需确认”的条目会触发分级确认，涉及存储切换时还会要求输入设置键名。"
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
                                                span { class: "settings-kv-badge is-warning", "需二次确认" }
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

pub fn render_general_fields(form: SettingsFormState, disabled: bool) -> Element {
    let mut site_name = form.site_name;
    let mut mail_enabled = form.mail_enabled;
    let mut mail_smtp_host = form.mail_smtp_host;
    let mut mail_smtp_port = form.mail_smtp_port;
    let mut mail_smtp_user = form.mail_smtp_user;
    let mut mail_smtp_password = form.mail_smtp_password;
    let mail_smtp_password_set = form.mail_smtp_password_set;
    let mut mail_from_email = form.mail_from_email;
    let mut mail_from_name = form.mail_from_name;
    let mut mail_link_base_url = form.mail_link_base_url;
    let mail_is_enabled = mail_enabled();
    let mail_jump_target = if mail_is_enabled {
        summary_value(mail_link_base_url())
    } else {
        "未启用".to_string()
    };

    rsx! {
        div { class: "settings-stack",
            div { class: "settings-status-summary",
                {render_metric_card("站点名称", summary_value(site_name()))}
                {render_metric_card("邮件服务", if mail_is_enabled { "已启用".to_string() } else { "未启用".to_string() })}
                {render_metric_card("站点访问地址", mail_jump_target)}
            }

            if mail_is_enabled {
                div { class: "settings-banner settings-banner-neutral",
                    "邮件服务已开启。公开注册、邮箱验证和密码找回都会依赖这里的 SMTP 与跳转地址配置。"
                }
            } else {
                div { class: "settings-banner settings-banner-neutral",
                    "邮件服务当前关闭。用户仍可登录，但公开注册后的邮箱验证和密码找回邮件不会发送。"
                }
            }

            div { class: "settings-subcard",
                h3 { "站点识别" }
                div { class: "settings-grid",
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

            div { class: "settings-subcard",
                h3 { "邮件投递" }
                div { class: "settings-grid",
                    label { class: "settings-check settings-field-full",
                        input {
                            r#type: "checkbox",
                            checked: mail_enabled(),
                            onchange: move |event| mail_enabled.set(event.checked()),
                            disabled,
                        }
                        span { "启用邮件服务" }
                    }

                    if mail_is_enabled {
                        label { class: "settings-field",
                            span { "发件邮箱" }
                            input {
                                r#type: "email",
                                value: "{mail_from_email()}",
                                oninput: move |event| mail_from_email.set(event.value()),
                                disabled,
                            }
                        }

                        label { class: "settings-field",
                            span { "发件人名称" }
                            input {
                                r#type: "text",
                                value: "{mail_from_name()}",
                                oninput: move |event| mail_from_name.set(event.value()),
                                disabled,
                            }
                        }

                        label { class: "settings-field settings-field-full",
                            span { "站点访问地址（用于邮件链接）" }
                            input {
                                r#type: "url",
                                value: "{mail_link_base_url()}",
                                oninput: move |event| mail_link_base_url.set(event.value()),
                                placeholder: "https://img.example.com",
                                disabled,
                            }
                            small { class: "settings-field-hint",
                                "用户点击邮件里的验证或重置链接后，会回到这里。"
                            }
                        }

                        label { class: "settings-field",
                            span { "SMTP 主机" }
                            input {
                                r#type: "text",
                                value: "{mail_smtp_host()}",
                                oninput: move |event| mail_smtp_host.set(event.value()),
                                disabled,
                            }
                        }

                        label { class: "settings-field",
                            span { "SMTP 端口" }
                            input {
                                r#type: "number",
                                min: "1",
                                value: "{mail_smtp_port()}",
                                oninput: move |event| mail_smtp_port.set(event.value()),
                                disabled,
                            }
                        }

                        label { class: "settings-field",
                            span { "SMTP 用户名" }
                            input {
                                r#type: "text",
                                value: "{mail_smtp_user()}",
                                oninput: move |event| mail_smtp_user.set(event.value()),
                                disabled,
                            }
                        }

                        label { class: "settings-field",
                            span {
                                if mail_smtp_password_set() {
                                    "SMTP 密码（留空不修改）"
                                } else {
                                    "SMTP 密码"
                                }
                            }
                            input {
                                r#type: "password",
                                value: "{mail_smtp_password()}",
                                oninput: move |event| mail_smtp_password.set(event.value()),
                                disabled,
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn render_general_fields_compact(form: SettingsFormState, disabled: bool) -> Element {
    let mut site_name = form.site_name;
    let mut mail_enabled = form.mail_enabled;
    let mut mail_smtp_host = form.mail_smtp_host;
    let mut mail_smtp_port = form.mail_smtp_port;
    let mut mail_smtp_user = form.mail_smtp_user;
    let mut mail_smtp_password = form.mail_smtp_password;
    let mail_smtp_password_set = form.mail_smtp_password_set;
    let mut mail_from_email = form.mail_from_email;
    let mut mail_from_name = form.mail_from_name;
    let mut mail_link_base_url = form.mail_link_base_url;
    let mail_is_enabled = mail_enabled();

    rsx! {
        div { class: "settings-stack",
            div { class: "settings-subcard install-compact-subcard",
                h3 { "站点信息" }
                div { class: "settings-grid",
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

            div { class: "settings-subcard install-compact-subcard",
                h3 { "邮件服务" }
                div { class: "settings-grid",
                    label { class: "settings-check settings-field-full",
                        input {
                            r#type: "checkbox",
                            checked: mail_enabled(),
                            onchange: move |event| mail_enabled.set(event.checked()),
                            disabled,
                        }
                        span { "启用邮件服务" }
                    }

                    if mail_is_enabled {
                        label { class: "settings-field",
                            span { "发件邮箱" }
                            input {
                                r#type: "email",
                                value: "{mail_from_email()}",
                                oninput: move |event| mail_from_email.set(event.value()),
                                disabled,
                            }
                        }

                        label { class: "settings-field",
                            span { "发件人名称" }
                            input {
                                r#type: "text",
                                value: "{mail_from_name()}",
                                oninput: move |event| mail_from_name.set(event.value()),
                                disabled,
                            }
                        }

                        label { class: "settings-field settings-field-full",
                            span { "站点访问地址（用于邮件链接）" }
                            input {
                                r#type: "url",
                                value: "{mail_link_base_url()}",
                                oninput: move |event| mail_link_base_url.set(event.value()),
                                placeholder: "https://img.example.com",
                                disabled,
                            }
                            small { class: "settings-field-hint",
                                "用户点击邮件里的验证或重置链接后，会回到这里。"
                            }
                        }

                        label { class: "settings-field",
                            span { "SMTP 主机" }
                            input {
                                r#type: "text",
                                value: "{mail_smtp_host()}",
                                oninput: move |event| mail_smtp_host.set(event.value()),
                                disabled,
                            }
                        }

                        label { class: "settings-field",
                            span { "SMTP 端口" }
                            input {
                                r#type: "number",
                                min: "1",
                                value: "{mail_smtp_port()}",
                                oninput: move |event| mail_smtp_port.set(event.value()),
                                disabled,
                            }
                        }

                        label { class: "settings-field",
                            span { "SMTP 用户名" }
                            input {
                                r#type: "text",
                                value: "{mail_smtp_user()}",
                                oninput: move |event| mail_smtp_user.set(event.value()),
                                disabled,
                            }
                        }

                        label { class: "settings-field",
                            span {
                                if mail_smtp_password_set() {
                                    "SMTP 密码（留空不修改）"
                                } else {
                                    "SMTP 密码"
                                }
                            }
                            input {
                                r#type: "password",
                                value: "{mail_smtp_password()}",
                                oninput: move |event| mail_smtp_password.set(event.value()),
                                disabled,
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn render_storage_fields(form: SettingsFormState, disabled: bool) -> Element {
    let mut storage_backend = form.storage_backend;
    let local_storage_path = form.local_storage_path;
    let show_s3_fields = form.is_s3_backend();
    let backend_label = if show_s3_fields {
        "对象存储".to_string()
    } else {
        "本地目录".to_string()
    };
    let bucket_summary = if show_s3_fields {
        summary_value((form.s3_bucket)())
    } else {
        "未启用".to_string()
    };

    rsx! {
        div { class: "settings-stack",
            div { class: "settings-status-summary",
                {render_metric_card("当前后端", backend_label)}
                {render_metric_card("本地目录", summary_value(local_storage_path()))}
                {render_metric_card("对象存储桶", bucket_summary)}
            }

            div { class: "settings-banner settings-banner-neutral",
                "存储后端切换属于运行时关键配置。保存后如果提示需要重启，请先确认目录或对象存储参数无误。"
            }

            div { class: "settings-subcard",
                h3 { "写入策略" }
                p { class: "settings-section-copy",
                    "本地模式适合单机部署；S3 模式适合对象存储或 MinIO。无论使用哪种方式，本地目录仍建议保留为可访问的数据卷路径。"
                }
                div { class: "settings-grid",
                    label { class: "settings-field",
                        span { "存储后端" }
                        select {
                            value: "{storage_backend()}",
                            onchange: move |event| storage_backend.set(event.value()),
                            disabled,
                            option { value: "local", "本地存储" }
                            option { value: "s3", "对象存储（S3）" }
                        }
                    }
                    if !show_s3_fields {
                        LocalStoragePathPicker { form, disabled }
                    }
                }
            }

            if show_s3_fields {
                div { class: "settings-subcard",
                    h3 { "对象存储参数" }
                    p { class: "settings-section-copy",
                        "切到 S3 后，后端将使用 endpoint / region / bucket / key 进行读写。MinIO 通常需要开启 path style。"
                    }
                    div { class: "settings-grid",
                        {render_s3_fields(form, disabled)}
                    }
                }
            }
        }
    }
}

pub fn render_storage_fields_compact(form: SettingsFormState, disabled: bool) -> Element {
    let mut storage_backend = form.storage_backend;
    let show_s3_fields = form.is_s3_backend();

    rsx! {
        div { class: "settings-stack",
            div { class: "settings-subcard install-compact-subcard",
                h3 { "存储后端" }
                div { class: "settings-grid",
                    label { class: "settings-field",
                        span { "存储后端" }
                        select {
                            value: "{storage_backend()}",
                            onchange: move |event| storage_backend.set(event.value()),
                            disabled,
                            option { value: "local", "本地存储" }
                            option { value: "s3", "对象存储（S3）" }
                        }
                    }
                    if !show_s3_fields {
                        LocalStoragePathPicker { form, disabled }
                    }
                }
            }

            if show_s3_fields {
                div { class: "settings-subcard install-compact-subcard",
                    h3 { "对象存储" }
                    div { class: "settings-grid",
                        {render_s3_fields(form, disabled)}
                    }
                }
            }
        }
    }
}

#[component]
fn LocalStoragePathPicker(form: SettingsFormState, disabled: bool) -> Element {
    let settings_service = use_settings_service();
    let auth_store = use_auth_store();
    let toast_store = use_toast_store();

    let mut local_storage_path = form.local_storage_path;
    let current_local_storage_path = local_storage_path();
    let requested_path = if current_local_storage_path.trim().is_empty() {
        DEFAULT_SETTINGS_STORAGE_BROWSER_PATH.to_string()
    } else {
        current_local_storage_path.clone()
    };

    let mut browser_open = use_signal(|| false);
    let browser_loading = use_signal(|| false);
    let browser_error = use_signal(String::new);
    let browser_current_path = use_signal(|| DEFAULT_SETTINGS_STORAGE_BROWSER_PATH.to_string());
    let browser_parent_path = use_signal(|| None::<String>);
    let browser_directories = use_signal(Vec::<StorageDirectoryEntry>::new);

    let open_browser_service = settings_service.clone();
    let browse_parent_service = settings_service.clone();
    let open_browser_auth_store = auth_store.clone();
    let browse_parent_auth_store = auth_store.clone();
    let open_browser_toast_store = toast_store.clone();
    let browse_parent_toast_store = toast_store.clone();
    let directory_entries = browser_directories();

    rsx! {
        div { class: "settings-field settings-field-full",
            span { "本地存储路径" }
            div { class: "install-path-picker",
                input {
                    class: "install-path-input",
                    r#type: "text",
                    value: "{current_local_storage_path}",
                    readonly: true,
                    disabled: true,
                }
                button {
                    class: "btn btn-ghost",
                    r#type: "button",
                    disabled: disabled || browser_loading(),
                    onclick: move |_| {
                        browser_open.set(true);
                        load_settings_storage_directories(
                            open_browser_service.clone(),
                            open_browser_auth_store.clone(),
                            open_browser_toast_store.clone(),
                            StorageBrowserSignals {
                                loading: browser_loading,
                                error: browser_error,
                                current_path: browser_current_path,
                                parent_path: browser_parent_path,
                                directories: browser_directories,
                            },
                            requested_path.clone(),
                        );
                    },
                    if browser_loading() { "读取中..." } else { "选择文件夹" }
                }
            }
            if browser_open() {
                Modal {
                    title: "选择本地存储目录".to_string(),
                    content_class: "storage-browser-modal-shell".to_string(),
                    on_close: move |_| browser_open.set(false),
                        div { class: "install-path-browser",
                            div { class: "install-path-browser-summary",
                                div {
                                    p { class: "install-path-browser-label", "当前目录" }
                                    code { class: "install-path-browser-current", "{browser_current_path()}" }
                                }
                                p { class: "install-path-browser-meta",
                                    if browser_loading() {
                                        "正在读取目录..."
                                    } else {
                                        "{directory_entries.len()} 个子目录"
                                    }
                                }
                            }
                            div { class: "install-path-browser-toolbar",
                                button {
                                    class: "btn btn-ghost",
                                    r#type: "button",
                                    disabled: disabled || browser_loading() || browser_parent_path().is_none(),
                                    onclick: move |_| {
                                        if let Some(parent_path) = browser_parent_path() {
                                        load_settings_storage_directories(
                                            browse_parent_service.clone(),
                                            browse_parent_auth_store.clone(),
                                            browse_parent_toast_store.clone(),
                                            StorageBrowserSignals {
                                                loading: browser_loading,
                                                error: browser_error,
                                                current_path: browser_current_path,
                                                parent_path: browser_parent_path,
                                                directories: browser_directories,
                                            },
                                            parent_path,
                                        );
                                        }
                                    },
                                    "上一级"
                                }
                                button {
                                    class: "btn btn-primary",
                                    r#type: "button",
                                    disabled: disabled,
                                    onclick: move |_| {
                                        local_storage_path.set(browser_current_path());
                                        browser_open.set(false);
                                    },
                                    "选择当前文件夹"
                                }
                            }
                            div { class: "install-path-browser-panel",
                                if !browser_error().is_empty() {
                                    p { class: "install-path-browser-error", "{browser_error()}" }
                                } else if browser_loading() {
                                    p { class: "install-path-browser-empty", "正在读取目录..." }
                                } else if directory_entries.is_empty() {
                                    p { class: "install-path-browser-empty", "当前目录下没有可继续展开的子目录。" }
                                } else {
                                    div { class: "install-path-browser-list",
                                        {directory_entries.iter().map(|entry| {
                                            let browse_entry_service = settings_service.clone();
                                            let browse_entry_auth_store = auth_store.clone();
                                            let browse_entry_toast_store = toast_store.clone();
                                            let entry_path = entry.path.clone();
                                            let entry_name = entry.name.clone();
                                            rsx! {
                                                button {
                                                    key: "{entry_path}",
                                                    class: "install-path-browser-item",
                                                    r#type: "button",
                                                    disabled: disabled || browser_loading(),
                                                    onclick: move |_| {
                                                        load_settings_storage_directories(
                                                            browse_entry_service.clone(),
                                                            browse_entry_auth_store.clone(),
                                                            browse_entry_toast_store.clone(),
                                                            StorageBrowserSignals {
                                                                loading: browser_loading,
                                                                error: browser_error,
                                                                current_path: browser_current_path,
                                                                parent_path: browser_parent_path,
                                                                directories: browser_directories,
                                                            },
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

fn load_settings_storage_directories(
    settings_service: SettingsService,
    auth_store: crate::store::AuthStore,
    toast_store: crate::store::ToastStore,
    browser_signals: StorageBrowserSignals,
    requested_path: String,
) {
    let requested_path = if requested_path.trim().is_empty() {
        DEFAULT_SETTINGS_STORAGE_BROWSER_PATH.to_string()
    } else {
        requested_path
    };

    spawn(async move {
        let StorageBrowserSignals {
            mut loading,
            error: mut error_signal,
            mut current_path,
            mut parent_path,
            mut directories,
        } = browser_signals;

        loading.set(true);
        error_signal.set(String::new());

        match settings_service
            .browse_storage_directories(Some(requested_path.as_str()))
            .await
        {
            Ok(response) => {
                current_path.set(response.current_path);
                parent_path.set(response.parent_path);
                directories.set(response.directories);
            }
            Err(err) => {
                if handle_settings_auth_error(&auth_store, &toast_store, &err) {
                    error_signal.set(settings_auth_expired_message());
                } else {
                    error_signal.set(format!("读取目录失败：{}", err));
                }
            }
        }

        loading.set(false);
    });
}

#[derive(Clone, Copy)]
struct StorageBrowserSignals {
    loading: Signal<bool>,
    error: Signal<String>,
    current_path: Signal<String>,
    parent_path: Signal<Option<String>>,
    directories: Signal<Vec<StorageDirectoryEntry>>,
}

pub fn render_s3_fields(form: SettingsFormState, disabled: bool) -> Element {
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
            span { "服务地址" }
            input {
                r#type: "text",
                value: "{s3_endpoint()}",
                oninput: move |event| s3_endpoint.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-field",
            span { "存储区域" }
            input {
                r#type: "text",
                value: "{s3_region()}",
                oninput: move |event| s3_region.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-field",
            span { "存储桶" }
            input {
                r#type: "text",
                value: "{s3_bucket()}",
                oninput: move |event| s3_bucket.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-field",
            span { "目录前缀（可选）" }
            input {
                r#type: "text",
                value: "{s3_prefix()}",
                oninput: move |event| s3_prefix.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-field",
            span { "访问密钥" }
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
                    "私有密钥（留空不修改）"
                } else {
                    "私有密钥"
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
            span { "使用路径风格（MinIO 通常需要开启）" }
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
    } else if status.eq_ignore_ascii_case("degraded") {
        "降级"
    } else if status.eq_ignore_ascii_case("disabled") {
        "已禁用"
    } else {
        "异常"
    }
}

fn status_surface_class(status: &str) -> &'static str {
    if status.eq_ignore_ascii_case("healthy") || status.eq_ignore_ascii_case("disabled") {
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
        .split(['_', '.'])
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

fn audit_title(log: &AuditLog) -> String {
    match log.action.as_str() {
        "admin.maintenance.deleted_files_cleanup.completed" | "cleanup_completed" => {
            "已清理历史已删除图片".to_string()
        }
        "admin.maintenance.deleted_files_cleanup.failed" | "cleanup_failed" => {
            "清理历史已删除图片失败".to_string()
        }
        "admin.maintenance.expired_images_cleanup.completed" | "expire_completed" => {
            "已永久删除过期图片".to_string()
        }
        "admin.maintenance.expired_images_cleanup.failed" | "expire_failed" => {
            "永久删除过期图片失败".to_string()
        }
        "admin.maintenance.database_backup.created" | "backup_created" => {
            "已创建数据库备份".to_string()
        }
        "admin.maintenance.database_backup.downloaded" => "已下载数据库备份".to_string(),
        "admin.maintenance.database_backup.deleted" => "已删除数据库备份".to_string(),
        "admin.maintenance.database_backup.delete_failed" => "删除数据库备份失败".to_string(),
        "admin.maintenance.database_backup.failed" => "数据库备份失败".to_string(),
        "admin.maintenance.database_restore.prechecked" => "数据库恢复预检已通过".to_string(),
        "admin.maintenance.database_restore.precheck_failed" => "数据库恢复预检未通过".to_string(),
        "admin.maintenance.database_restore.scheduled" => "已写入数据库恢复计划".to_string(),
        "system.database_restore.completed" => "数据库恢复已完成".to_string(),
        "system.database_restore.rollback_applied" => "数据库恢复后已自动回滚".to_string(),
        "system.database_restore.failed" => "数据库恢复失败".to_string(),
        "admin.user.role_updated" => "已更新用户角色".to_string(),
        "admin.settings.config_updated" => "已保存系统设置".to_string(),
        "admin.settings.raw_setting_updated" => "已更新原始设置项".to_string(),
        "system.install_completed" => "已完成安装向导".to_string(),
        "image.upload" => "已上传图片".to_string(),
        "image.view" => "已访问图片".to_string(),
        "user.login" => "用户已登录".to_string(),
        _ => humanize_action(&log.action),
    }
}

fn audit_summary(log: &AuditLog) -> String {
    match log.action.as_str() {
        "admin.maintenance.deleted_files_cleanup.completed" | "cleanup_completed" => {
            if let Some(count) = audit_detail_i64(log.details.as_ref(), "removed_count") {
                format!("共清理 {} 个历史已删除文件。", count)
            } else {
                "已完成历史已删除图片清理。".to_string()
            }
        }
        "admin.maintenance.deleted_files_cleanup.failed" | "cleanup_failed" => {
            audit_error_summary(log)
                .unwrap_or_else(|| "历史已删除图片清理失败，请检查数据库与存储状态。".to_string())
        }
        "admin.maintenance.expired_images_cleanup.completed" | "expire_completed" => {
            if let Some(count) = audit_detail_i64(log.details.as_ref(), "affected_count") {
                format!("共永久删除 {} 张已过期图片。", count)
            } else {
                "已永久删除所有符合条件的过期图片。".to_string()
            }
        }
        "admin.maintenance.expired_images_cleanup.failed" | "expire_failed" => {
            audit_error_summary(log)
                .unwrap_or_else(|| "过期图片永久删除失败，请检查图片数据与数据库状态。".to_string())
        }
        "admin.maintenance.database_backup.created" | "backup_created" => {
            audit_detail_str(log.details.as_ref(), "filename")
                .map(|filename| format!("备份文件已生成：{}。", filename))
                .unwrap_or_else(|| "数据库备份已生成。".to_string())
        }
        "admin.maintenance.database_backup.downloaded" => {
            let filename = audit_detail_str(log.details.as_ref(), "filename")
                .unwrap_or_else(|| "数据库备份".to_string());
            match audit_detail_i64(log.details.as_ref(), "size_bytes") {
                Some(size_bytes) => format!(
                    "已下载备份文件 {}，大小 {}。",
                    filename,
                    format_storage_bytes(size_bytes)
                ),
                None => format!("已下载备份文件 {}。", filename),
            }
        }
        "admin.maintenance.database_backup.deleted" => {
            let filename = audit_detail_str(log.details.as_ref(), "filename")
                .unwrap_or_else(|| "数据库备份".to_string());
            match audit_detail_i64(log.details.as_ref(), "size_bytes") {
                Some(size_bytes) => format!(
                    "已删除备份文件 {}，释放 {}。",
                    filename,
                    format_storage_bytes(size_bytes)
                ),
                None => format!("已删除备份文件 {}。", filename),
            }
        }
        "admin.maintenance.database_backup.delete_failed" => audit_error_summary(log)
            .unwrap_or_else(|| {
                "删除数据库备份失败，请检查文件是否仍存在以及备份目录权限。".to_string()
            }),
        "admin.maintenance.database_backup.failed" => {
            audit_error_summary(log).unwrap_or_else(|| {
                "数据库备份失败，请检查备份目录、数据库连接和导出命令。".to_string()
            })
        }
        "admin.maintenance.database_restore.prechecked" => {
            let database_kind = audit_detail_str(log.details.as_ref(), "backup_database_kind")
                .unwrap_or_else(|| "sqlite".to_string());
            let database_label = restore_database_kind_label(&database_kind);
            let filename = audit_detail_str(log.details.as_ref(), "filename")
                .unwrap_or_else(|| format!("{database_label} 备份"));
            let warnings = audit_detail_string_list(log.details.as_ref(), "warnings");
            if warnings.is_empty() {
                format!(
                    "备份 {} 已通过恢复预检，可以在下一次重启前执行恢复。",
                    filename
                )
            } else {
                format!(
                    "备份 {} 已通过恢复预检；注意事项：{}。",
                    filename,
                    warnings.join("；")
                )
            }
        }
        "admin.maintenance.database_restore.precheck_failed" => {
            let database_kind = audit_detail_str(log.details.as_ref(), "backup_database_kind")
                .unwrap_or_else(|| "sqlite".to_string());
            let database_label = restore_database_kind_label(&database_kind);
            let filename = audit_detail_str(log.details.as_ref(), "filename")
                .unwrap_or_else(|| format!("{database_label} 备份"));
            let blockers = audit_detail_string_list(log.details.as_ref(), "blockers");
            if blockers.is_empty() {
                format!("备份 {} 未通过恢复预检。", filename)
            } else {
                format!(
                    "备份 {} 未通过恢复预检：{}。",
                    filename,
                    blockers.join("；")
                )
            }
        }
        "admin.maintenance.database_restore.scheduled" => {
            let database_kind = audit_detail_str(log.details.as_ref(), "database_kind")
                .unwrap_or_else(|| "sqlite".to_string());
            let database_label = restore_database_kind_label(&database_kind);
            let filename = audit_detail_str(log.details.as_ref(), "filename")
                .unwrap_or_else(|| format!("{database_label} 备份"));
            format!(
                "{} 的恢复计划已写入；需要尽快重启服务，真正恢复会在下一次启动前执行。",
                filename
            )
        }
        "system.database_restore.completed" => {
            let database_kind = audit_detail_str(log.details.as_ref(), "database_kind")
                .unwrap_or_else(|| "sqlite".to_string());
            let database_label = restore_database_kind_label(&database_kind);
            let filename = audit_detail_str(log.details.as_ref(), "filename")
                .unwrap_or_else(|| format!("{database_label} 备份"));
            let rollback_filename = audit_detail_str(log.details.as_ref(), "rollback_filename");
            match rollback_filename {
                Some(rollback) => format!(
                    "备份 {} 已完成恢复，同时保留回滚快照 {} 以便必要时手工追溯。",
                    filename, rollback
                ),
                None => format!("备份 {} 已在启动前完成恢复。", filename),
            }
        }
        "system.database_restore.rollback_applied" => {
            audit_detail_str(log.details.as_ref(), "message").unwrap_or_else(|| {
                "恢复后的数据库启动失败，系统已自动回滚到恢复前快照。".to_string()
            })
        }
        "system.database_restore.failed" => audit_detail_str(log.details.as_ref(), "message")
            .unwrap_or_else(|| "数据库恢复失败，请检查最后一次恢复结果与启动日志。".to_string()),
        "admin.user.role_updated" => {
            let email = audit_detail_str(log.details.as_ref(), "user_email")
                .unwrap_or_else(|| "目标用户".to_string());
            let previous_role = audit_detail_str(log.details.as_ref(), "previous_role")
                .unwrap_or_else(|| "unknown".to_string());
            let new_role = audit_detail_str(log.details.as_ref(), "new_role")
                .unwrap_or_else(|| "unknown".to_string());
            format!(
                "{} 的角色已从 {} 调整为 {}。",
                email,
                role_label(&previous_role),
                role_label(&new_role)
            )
        }
        "admin.settings.config_updated" => {
            let changed_keys = audit_detail_string_list(log.details.as_ref(), "changed_keys")
                .into_iter()
                .map(|key| setting_key_label(&key).to_string())
                .collect::<Vec<_>>();
            let restart_required = audit_detail_bool(log.details.as_ref(), "restart_required");
            if changed_keys.is_empty() {
                "已保存结构化系统设置。".to_string()
            } else if restart_required {
                format!(
                    "已更新 {}，且这些调整需要重启服务后完全生效。",
                    changed_keys.join("、")
                )
            } else {
                format!("已更新 {}。", changed_keys.join("、"))
            }
        }
        "admin.settings.raw_setting_updated" => {
            let key = audit_detail_str(log.details.as_ref(), "setting_key")
                .map(|value| setting_key_label(&value).to_string())
                .unwrap_or_else(|| "未知键名".to_string());
            let restart_required = audit_detail_bool(log.details.as_ref(), "restart_required");
            if restart_required {
                format!("已通过高级设置修改 {}，需要重启服务后完全生效。", key)
            } else {
                format!("已通过高级设置修改 {}。", key)
            }
        }
        "system.install_completed" => {
            let site_name = audit_detail_str(log.details.as_ref(), "site_name")
                .unwrap_or_else(|| "站点".to_string());
            let storage_backend = audit_detail_str(log.details.as_ref(), "storage_backend")
                .unwrap_or_else(|| "local".to_string());
            let storage_label = if storage_backend.eq_ignore_ascii_case("s3") {
                "对象存储"
            } else {
                "本地目录"
            };
            let mail_status = if audit_detail_bool(log.details.as_ref(), "mail_enabled") {
                "邮件已启用"
            } else {
                "邮件未启用"
            };
            let favicon_status = if audit_detail_bool(log.details.as_ref(), "favicon_configured") {
                "图标已配置"
            } else {
                "图标未配置"
            };
            format!(
                "安装完成，站点名称为 {}，存储使用 {}，{}，{}。",
                site_name, storage_label, mail_status, favicon_status
            )
        }
        "image.upload" => {
            let filename = audit_detail_str(log.details.as_ref(), "original_filename")
                .or_else(|| audit_detail_str(log.details.as_ref(), "stored_filename"))
                .unwrap_or_else(|| "未命名图片".to_string());
            let mut segments = vec![format!("已上传 {}", filename)];
            if let Some(format) = audit_detail_str(log.details.as_ref(), "format") {
                segments.push(format!("格式 {}", format.to_uppercase()));
            }
            if let Some(size_bytes) = audit_detail_i64(log.details.as_ref(), "size_bytes") {
                segments.push(format!("大小 {}", format_storage_bytes(size_bytes)));
            }
            format!("{}。", segments.join("，"))
        }
        "image.view" => {
            if let Some(target_id) = log.target_id.as_deref() {
                format!(
                    "已记录一次图片访问，目标ID 为 {}。",
                    short_identifier(target_id)
                )
            } else {
                "已记录一次图片访问。".to_string()
            }
        }
        "user.login" => audit_detail_str(log.details.as_ref(), "email")
            .map(|email| format!("{email} 已登录控制台。"))
            .unwrap_or_else(|| "当前账户已登录控制台。".to_string()),
        _ => log
            .details
            .as_ref()
            .map(format_json_details)
            .unwrap_or_else(|| "无附加详情".to_string()),
    }
}

fn audit_category_label(log: &AuditLog) -> &'static str {
    match log.action.as_str() {
        "admin.maintenance.deleted_files_cleanup.completed"
        | "admin.maintenance.deleted_files_cleanup.failed"
        | "cleanup_completed"
        | "cleanup_failed"
        | "admin.maintenance.expired_images_cleanup.completed"
        | "admin.maintenance.expired_images_cleanup.failed"
        | "expire_completed"
        | "expire_failed"
        | "admin.maintenance.database_backup.created"
        | "admin.maintenance.database_backup.deleted"
        | "admin.maintenance.database_backup.delete_failed"
        | "admin.maintenance.database_backup.failed"
        | "admin.maintenance.database_backup.downloaded"
        | "admin.maintenance.database_restore.prechecked"
        | "admin.maintenance.database_restore.precheck_failed"
        | "admin.maintenance.database_restore.scheduled"
        | "system.database_restore.completed"
        | "system.database_restore.rollback_applied"
        | "system.database_restore.failed"
        | "backup_created" => "维护操作",
        "admin.user.role_updated" => "权限变更",
        "admin.settings.config_updated" | "admin.settings.raw_setting_updated" => "设置变更",
        "system.install_completed" => "安装初始化",
        "image.upload" | "image.view" => "图片操作",
        "user.login" => "认证事件",
        _ => "系统事件",
    }
}

fn audit_category_class(log: &AuditLog) -> &'static str {
    match audit_risk_value(log).as_deref() {
        Some("danger") => "is-danger",
        Some("warning") => "is-warning",
        Some("info") => "is-info",
        _ => "is-neutral",
    }
}

fn audit_risk_label(log: &AuditLog) -> &'static str {
    match audit_risk_value(log).as_deref() {
        Some("danger") => "高风险",
        Some("warning") => "需确认",
        Some("info") => "常规",
        _ => "事件",
    }
}

fn audit_risk_value(log: &AuditLog) -> Option<String> {
    audit_detail_str(log.details.as_ref(), "risk_level").or_else(|| match log.action.as_str() {
        "admin.maintenance.deleted_files_cleanup.completed"
        | "admin.maintenance.deleted_files_cleanup.failed"
        | "cleanup_completed"
        | "cleanup_failed"
        | "admin.maintenance.database_backup.deleted"
        | "admin.maintenance.database_backup.delete_failed"
        | "admin.maintenance.database_restore.prechecked"
        | "admin.maintenance.database_restore.precheck_failed"
        | "admin.maintenance.database_restore.scheduled"
        | "system.database_restore.completed"
        | "system.database_restore.rollback_applied"
        | "system.database_restore.failed"
        | "admin.user.role_updated" => Some("danger".to_string()),
        "admin.maintenance.expired_images_cleanup.completed"
        | "admin.maintenance.expired_images_cleanup.failed"
        | "expire_completed"
        | "expire_failed"
        | "admin.settings.config_updated"
        | "admin.settings.raw_setting_updated" => Some("warning".to_string()),
        "admin.maintenance.database_backup.created"
        | "admin.maintenance.database_backup.failed"
        | "admin.maintenance.database_backup.downloaded"
        | "backup_created"
        | "system.install_completed"
        | "image.upload"
        | "image.view"
        | "user.login" => Some("info".to_string()),
        _ => None,
    })
}

fn is_bootstrap_admin_id(value: &str) -> bool {
    value == "00000000-0000-0000-0000-000000000001"
}

fn is_maintenance_action(action: &str, target_type: &str) -> bool {
    matches!(
        action,
        "admin.maintenance.deleted_files_cleanup.completed"
            | "admin.maintenance.deleted_files_cleanup.failed"
            | "cleanup_completed"
            | "cleanup_failed"
            | "admin.maintenance.expired_images_cleanup.completed"
            | "admin.maintenance.expired_images_cleanup.failed"
            | "expire_completed"
            | "expire_failed"
            | "admin.maintenance.database_backup.created"
            | "admin.maintenance.database_backup.deleted"
            | "admin.maintenance.database_backup.delete_failed"
            | "admin.maintenance.database_backup.failed"
            | "admin.maintenance.database_backup.downloaded"
            | "admin.maintenance.database_restore.prechecked"
            | "admin.maintenance.database_restore.precheck_failed"
            | "admin.maintenance.database_restore.scheduled"
            | "system.database_restore.completed"
            | "system.database_restore.rollback_applied"
            | "system.database_restore.failed"
            | "backup_created"
    ) || target_type == "maintenance"
}

fn audit_target_type_label(target_type: &str) -> String {
    match target_type {
        "maintenance" => "维护任务".to_string(),
        "settings" => "系统设置".to_string(),
        "setting" => "原始设置项".to_string(),
        "user" => "用户".to_string(),
        "image" => "图片".to_string(),
        "system" => "系统".to_string(),
        _ => humanize_action(target_type),
    }
}

fn audit_actor_label(log: &AuditLog) -> String {
    audit_detail_str(log.details.as_ref(), "admin_email")
        .or_else(|| audit_detail_str(log.details.as_ref(), "actor_email"))
        .or_else(|| audit_detail_str(log.details.as_ref(), "email"))
        .or_else(|| {
            if log.action == "user.login" {
                Some("当前用户".to_string())
            } else if log.action == "system.install_completed" {
                Some("安装向导".to_string())
            } else if is_maintenance_action(&log.action, &log.target_type)
                && log.user_id.as_deref().is_some_and(is_bootstrap_admin_id)
            {
                Some("系统任务".to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| optional_short_id(log.user_id.as_deref()))
}

fn audit_error_summary(log: &AuditLog) -> Option<String> {
    audit_detail_str(log.details.as_ref(), "error").map(|error| format!("错误信息：{}。", error))
}

fn audit_detail_str(details: Option<&serde_json::Value>, key: &str) -> Option<String> {
    details?.get(key)?.as_str().map(|value| value.to_string())
}

fn audit_detail_i64(details: Option<&serde_json::Value>, key: &str) -> Option<i64> {
    details?.get(key)?.as_i64()
}

fn audit_detail_bool(details: Option<&serde_json::Value>, key: &str) -> bool {
    details
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn audit_detail_string_list(details: Option<&serde_json::Value>, key: &str) -> Vec<String> {
    details
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(|value| value.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn setting_key_label(key: &str) -> String {
    match key {
        "site_name" => "网站名称".to_string(),
        "storage_backend" => "存储后端".to_string(),
        "local_storage_path" => "本地存储路径".to_string(),
        "mail_enabled" => "邮件服务开关".to_string(),
        "mail_smtp_host" => "SMTP 主机".to_string(),
        "mail_smtp_port" => "SMTP 端口".to_string(),
        "mail_smtp_user" => "SMTP 用户名".to_string(),
        "mail_smtp_password" => "SMTP 密码".to_string(),
        "mail_from_email" => "发件邮箱".to_string(),
        "mail_from_name" => "发件人名称".to_string(),
        "mail_link_base_url" => "站点访问地址（邮件链接）".to_string(),
        "s3_endpoint" => "对象存储服务地址".to_string(),
        "s3_region" => "对象存储区域".to_string(),
        "s3_bucket" => "对象存储桶".to_string(),
        "s3_prefix" => "对象存储目录前缀".to_string(),
        "s3_access_key" => "对象存储访问密钥".to_string(),
        "s3_secret_key" => "对象存储私有密钥".to_string(),
        "s3_force_path_style" => "对象存储路径风格".to_string(),
        _ => key.to_string(),
    }
}

fn render_audit_log_card(log: AuditLog) -> Element {
    let audit_title = audit_title(&log);
    let audit_summary = audit_summary(&log);
    let audit_category = audit_category_label(&log);
    let audit_category_class = audit_category_class(&log);
    let audit_risk = audit_risk_label(&log);
    let target_type = audit_target_type_label(&log.target_type);
    let actor = audit_actor_label(&log);

    rsx! {
        article { class: "settings-log-card",
            div { class: "settings-log-head",
                div { class: "settings-log-title",
                    h3 { "{audit_title}" }
                    p { class: "settings-log-meta",
                        "{format_timestamp(log.created_at)} · 目标 {target_type}"
                    }
                    p { class: "settings-log-summary", "{audit_summary}" }
                }
                span { class: format!("settings-log-tag {}", audit_category_class), "{audit_category}" }
            }

            div { class: "settings-log-meta-grid",
                span { class: format!("settings-log-chip {}", audit_category_class), "{audit_risk}" }
                span { class: "settings-log-chip", "{log.action}" }
                span { class: "settings-log-chip", "操作者 {actor}" }
                span { class: "settings-log-chip", "目标ID {optional_short_id(log.target_id.as_deref())}" }
                span { class: "settings-log-chip", {"IP ".to_string() + &log.ip_address.clone().unwrap_or_else(|| "未知".to_string())} }
            }

            if let Some(details) = &log.details {
                pre { class: "settings-code-block", "{format_json_details(details)}" }
            }
        }
    }
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

fn format_storage_bytes_u64(bytes: u64) -> String {
    if bytes > i64::MAX as u64 {
        format!("{} B", bytes)
    } else {
        format_storage_bytes(bytes as i64)
    }
}

fn backup_kind_label(semantics: &BackupSemantics) -> &'static str {
    match semantics.backup_kind.as_str() {
        "sqlite-database-snapshot" => "SQLite 数据库快照",
        "mysql-logical-dump" => "MySQL / MariaDB 逻辑导出",
        "postgresql-logical-dump" => "PostgreSQL 逻辑导出",
        _ => restore_database_kind_label(&semantics.database_family),
    }
}

fn backup_supports_restore(semantics: &BackupSemantics) -> bool {
    semantics.ui_restore_supported
}

fn restore_database_kind_label(kind: &str) -> &'static str {
    if kind.eq_ignore_ascii_case("sqlite") {
        "SQLite"
    } else if kind.eq_ignore_ascii_case("mysql") {
        "MySQL / MariaDB"
    } else if kind.eq_ignore_ascii_case("postgresql") || kind.eq_ignore_ascii_case("postgres") {
        "PostgreSQL"
    } else {
        "数据库"
    }
}

fn restore_status_label(status: &str) -> &'static str {
    match status {
        "completed" => "已完成",
        "rolled_back" => "已回滚",
        "failed" => "失败",
        "pending" => "待执行",
        _ => "未知",
    }
}

fn restore_status_surface_class(status: &str) -> &'static str {
    match status {
        "completed" => "is-info",
        "rolled_back" | "failed" => "is-danger",
        "pending" => "is-warning",
        _ => "",
    }
}

fn format_storage_mb(storage_used_mb: Option<f64>) -> String {
    storage_used_mb
        .map(|value| format!("{value:.2} MB"))
        .unwrap_or_else(|| "未知".to_string())
}

fn summary_value(value: String) -> String {
    let value = value.trim().to_string();
    if value.is_empty() {
        "未配置".to_string()
    } else {
        value
    }
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
