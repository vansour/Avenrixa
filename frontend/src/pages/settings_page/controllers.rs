use crate::app_context::{use_admin_service, use_auth_service, use_auth_store, use_toast_store};
use crate::components::{ConfirmationModal, ConfirmationTone};
use crate::store::{AuthStore, ToastStore};
use crate::types::api::{
    AdminUserSummary, AuditLog, BackupFileSummary, BackupObjectRollbackAnchor, BackupResponse,
    BackupRestorePrecheckResponse, BackupRestoreStatusResponse, BackupRestoreStorageSummary,
    BackupSemantics, HealthStatus, PaginationParams, Setting, SystemStats, UpdateProfileRequest,
};
use crate::types::errors::AppError;
use dioxus::prelude::*;
use std::collections::HashMap;

use super::view::{
    AccountSettingsSection, AdvancedSettingsSection, AuditSettingsSection,
    MaintenanceSettingsSection, SecuritySettingsSection, SystemStatusSection, UsersSettingsSection,
};
use super::{handle_settings_auth_error, settings_auth_expired_message};

#[derive(Clone, PartialEq, Eq)]
struct ConfirmationPlan {
    title: String,
    summary: String,
    consequences: Vec<String>,
    confirm_label: String,
    cancel_label: String,
    tone: ConfirmationTone,
    confirm_phrase: Option<String>,
    confirm_hint: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
enum MaintenanceAction {
    CleanupDeleted,
    CleanupExpired,
    DeleteBackup(String),
    RestoreBackup(String),
}

#[derive(Clone, PartialEq, Eq)]
struct PendingMaintenanceAction {
    action: MaintenanceAction,
    plan: ConfirmationPlan,
}

#[derive(Clone, PartialEq, Eq)]
struct PendingUserRoleChange {
    user_id: String,
    email: String,
    next_role: String,
    plan: ConfirmationPlan,
}

#[derive(Clone, PartialEq, Eq)]
struct PendingSettingChange {
    key: String,
    next_value: String,
    plan: ConfirmationPlan,
}

fn advanced_setting_confirm_message(key: &str) -> &'static str {
    match key {
        "storage_backend" => {
            "确认修改 storage_backend 吗？保存后需要重启服务，后端才会切换存储方式。"
        }
        "local_storage_path" => {
            "确认修改 local_storage_path 吗？保存后需要重启服务，新的写入目录才会生效。"
        }
        "s3_endpoint" | "s3_region" | "s3_bucket" | "s3_prefix" => {
            "确认修改这项 S3 设置吗？保存后需要重启服务，错误配置会影响对象存储读写。"
        }
        "s3_force_path_style" => {
            "确认修改 s3_force_path_style 吗？保存后需要重启服务，这会影响 S3/MinIO 的访问方式。"
        }
        _ => "确认保存这项高级设置吗？",
    }
}

fn setting_requires_restart(key: &str) -> bool {
    matches!(
        key,
        "storage_backend"
            | "local_storage_path"
            | "s3_endpoint"
            | "s3_region"
            | "s3_bucket"
            | "s3_prefix"
            | "s3_force_path_style"
    )
}

fn maintenance_confirmation_plan(action: MaintenanceAction) -> ConfirmationPlan {
    match action {
        MaintenanceAction::CleanupDeleted => ConfirmationPlan {
            title: "永久清理已删除文件".to_string(),
            summary: "这会从存储和数据库记录中彻底移除已删除文件，执行后无法恢复。".to_string(),
            consequences: vec![
                "只处理已经进入删除状态的文件和记录。".to_string(),
                "一旦执行完成，这批文件将不能再从回收站恢复。".to_string(),
                "建议在清理前先做数据库备份。".to_string(),
            ],
            confirm_label: "确认永久清理".to_string(),
            cancel_label: "取消".to_string(),
            tone: ConfirmationTone::Danger,
            confirm_phrase: Some("DELETE".to_string()),
            confirm_hint: Some("请输入 DELETE 以确认永久清理".to_string()),
        },
        MaintenanceAction::CleanupExpired => ConfirmationPlan {
            title: "批量处理过期图片".to_string(),
            summary: "系统会把所有已过期图片批量移入回收站，后续仍可在回收站恢复。".to_string(),
            consequences: vec![
                "只影响已经到期的图片。".to_string(),
                "处理后图片不会立刻永久删除，而是进入回收站。".to_string(),
                "如果过期策略配置不正确，可能一次影响较多图片。".to_string(),
            ],
            confirm_label: "继续处理".to_string(),
            cancel_label: "取消".to_string(),
            tone: ConfirmationTone::Warning,
            confirm_phrase: None,
            confirm_hint: None,
        },
        MaintenanceAction::DeleteBackup(filename) => ConfirmationPlan {
            title: "删除备份文件".to_string(),
            summary: format!("你正在删除备份文件 {}。", filename),
            consequences: vec![
                "只会删除备份目录中的快照文件，不会直接修改当前在线数据库。".to_string(),
                "删除后这份快照将不能再下载，也不能再作为回滚依据。".to_string(),
                "建议仅删除已确认不再需要的旧备份。".to_string(),
            ],
            confirm_label: "确认删除备份".to_string(),
            cancel_label: "取消".to_string(),
            tone: ConfirmationTone::Danger,
            confirm_phrase: Some(filename.clone()),
            confirm_hint: Some(format!("请输入 {} 以确认删除", filename)),
        },
        MaintenanceAction::RestoreBackup(_) => unreachable!("restore confirmation uses precheck"),
    }
}

fn backup_supports_restore(semantics: &BackupSemantics) -> bool {
    semantics.ui_restore_supported
}

fn format_restore_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn format_restore_timestamp(timestamp: chrono::DateTime<chrono::Utc>) -> String {
    timestamp.format("%Y-%m-%d %H:%M UTC").to_string()
}

fn summarize_restore_storage(storage: &BackupRestoreStorageSummary) -> String {
    if storage.storage_backend.eq_ignore_ascii_case("s3") {
        let bucket = storage
            .s3_bucket
            .clone()
            .unwrap_or_else(|| "未配置 bucket".to_string());
        let endpoint = storage
            .s3_endpoint
            .clone()
            .unwrap_or_else(|| "未配置 endpoint".to_string());
        format!("对象存储 · {} · {}", bucket, endpoint)
    } else {
        format!("本地目录 · {}", storage.local_storage_path)
    }
}

fn database_kind_label(kind: &str) -> &'static str {
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

fn backup_kind_label(semantics: &BackupSemantics) -> &'static str {
    match semantics.backup_kind.as_str() {
        "sqlite-database-snapshot" => "SQLite 数据库快照",
        "mysql-logical-dump" => "MySQL / MariaDB 逻辑导出",
        "postgresql-logical-dump" => "PostgreSQL 逻辑导出",
        _ => database_kind_label(&semantics.database_family),
    }
}

fn restore_mode_label(semantics: &BackupSemantics) -> &'static str {
    match semantics.restore_mode.as_str() {
        "ui-restart-file-swap" => "重启前文件替换恢复",
        "ui-restart-sql-import" => "重启前导入恢复",
        "ops-tooling-only" => "仅运维脚本恢复",
        "download-only" => "仅下载，不支持页面恢复",
        _ => "恢复方式未知",
    }
}

fn summarize_object_rollback_anchor(anchor: &BackupObjectRollbackAnchor) -> String {
    if anchor.strategy == "local-directory-snapshot" {
        let path = anchor
            .local_storage_path
            .clone()
            .unwrap_or_else(|| "未记录目录".to_string());
        format!(
            "文件回滚锚点：{} @ {}",
            path,
            format_restore_timestamp(anchor.checkpoint_at)
        )
    } else {
        let bucket = anchor
            .s3_bucket
            .clone()
            .unwrap_or_else(|| "未配置 bucket".to_string());
        let prefix = anchor.s3_prefix.clone().unwrap_or_else(|| "/".to_string());
        let status = anchor
            .s3_bucket_versioning_status
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        format!(
            "对象回滚锚点：bucket {} · prefix {} · {} · versioning {}",
            bucket,
            prefix,
            format_restore_timestamp(anchor.checkpoint_at),
            status
        )
    }
}

fn restore_confirmation_plan(precheck: &BackupRestorePrecheckResponse) -> ConfirmationPlan {
    let mut consequences = vec![
        format!(
            "目标备份创建于 {}，大小 {}。",
            format_restore_timestamp(precheck.backup_created_at),
            format_restore_bytes(precheck.backup_size_bytes)
        ),
        format!(
            "当前数据库后端：{}；目标备份类型：{}。",
            database_kind_label(&precheck.current_database_kind),
            backup_kind_label(&precheck.semantics)
        ),
        format!(
            "当前恢复方式：{}。",
            restore_mode_label(&precheck.semantics)
        ),
        "恢复不是在线热切换；真正的数据替换或 SQL 导入会发生在下一次服务启动前。".to_string(),
        format!(
            "当前存储配置：{}。",
            summarize_restore_storage(&precheck.current_storage)
        ),
        format!(
            "备份内存储配置：{}。",
            summarize_restore_storage(&precheck.backup_storage)
        ),
    ];
    if let Some(anchor) = precheck.object_rollback_anchor.as_ref() {
        consequences.push(summarize_object_rollback_anchor(anchor));
    }
    consequences.extend(precheck.warnings.iter().cloned());

    let database_label = database_kind_label(&precheck.backup_database_kind);
    ConfirmationPlan {
        title: format!("写入 {database_label} 恢复计划"),
        summary: format!(
            "你正在安排在下一次重启时，用备份 {} 恢复当前 {}。执行后当前登录会话会全部失效，需要重新登录。",
            precheck.filename, database_label
        ),
        consequences,
        confirm_label: "写入恢复计划".to_string(),
        cancel_label: "取消".to_string(),
        tone: ConfirmationTone::Danger,
        confirm_phrase: Some(precheck.filename.clone()),
        confirm_hint: Some(format!("请输入 {} 以确认写入恢复计划", precheck.filename)),
    }
}

fn restore_precheck_error_message(precheck: &BackupRestorePrecheckResponse) -> String {
    if precheck.blockers.is_empty() {
        format!("备份 {} 当前不能恢复，请检查服务状态。", precheck.filename)
    } else {
        format!(
            "备份 {} 当前不能恢复：{}",
            precheck.filename,
            precheck.blockers.join("；")
        )
    }
}

fn role_change_confirmation_plan(
    email: &str,
    current_role: &str,
    next_role: &str,
) -> ConfirmationPlan {
    if current_role.eq_ignore_ascii_case("admin") && next_role.eq_ignore_ascii_case("user") {
        ConfirmationPlan {
            title: "降级管理员权限".to_string(),
            summary: format!("你正在把 {} 从管理员降级为普通用户。", email),
            consequences: vec![
                "该用户将失去系统设置、用户管理和维护工具访问权限。".to_string(),
                "如果这是最后一个管理员，后续将无法再通过界面管理系统。".to_string(),
                "建议先确认仍有其他管理员账户可用。".to_string(),
            ],
            confirm_label: "确认降级".to_string(),
            cancel_label: "取消".to_string(),
            tone: ConfirmationTone::Danger,
            confirm_phrase: Some(email.to_string()),
            confirm_hint: Some(format!("请输入 {} 以确认降级", email)),
        }
    } else {
        ConfirmationPlan {
            title: "提升用户权限".to_string(),
            summary: format!("你正在把 {} 提升为管理员。", email),
            consequences: vec![
                "该用户将获得系统设置、用户管理和维护工具访问权限。".to_string(),
                "管理员可以修改底层配置并执行高风险维护操作。".to_string(),
            ],
            confirm_label: "确认提升".to_string(),
            cancel_label: "取消".to_string(),
            tone: ConfirmationTone::Warning,
            confirm_phrase: None,
            confirm_hint: None,
        }
    }
}

fn advanced_setting_confirmation_plan(key: &str, next_value: &str) -> ConfirmationPlan {
    let next_value_preview = if next_value.trim().is_empty() {
        "空值".to_string()
    } else {
        format!("新值将更新为：{}", truncate_for_confirmation(next_value))
    };

    if setting_requires_restart(key) {
        ConfirmationPlan {
            title: format!("确认修改 {}", key),
            summary: format!(
                "{}。这类底层存储配置通常需要重启服务才能完全生效。",
                advanced_setting_confirm_message(key)
            ),
            consequences: vec![
                next_value_preview,
                "错误配置会影响上传、读取或对象存储访问。".to_string(),
                "保存后建议立刻检查健康状态并重启服务。".to_string(),
            ],
            confirm_label: "确认保存".to_string(),
            cancel_label: "取消".to_string(),
            tone: ConfirmationTone::Danger,
            confirm_phrase: Some(key.to_string()),
            confirm_hint: Some(format!("请输入 {} 以确认修改", key)),
        }
    } else {
        ConfirmationPlan {
            title: format!("确认更新 {}", key),
            summary: advanced_setting_confirm_message(key).to_string(),
            consequences: vec![
                next_value_preview,
                "这会直接写入底层 settings 表。".to_string(),
            ],
            confirm_label: "继续保存".to_string(),
            cancel_label: "取消".to_string(),
            tone: ConfirmationTone::Warning,
            confirm_phrase: None,
            confirm_hint: None,
        }
    }
}

fn truncate_for_confirmation(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() > 60 {
        format!("{}...", trimmed.chars().take(60).collect::<String>())
    } else {
        trimmed.to_string()
    }
}

fn merge_messages(primary: &str, secondary: &str) -> String {
    match (primary.trim(), secondary.trim()) {
        ("", "") => String::new(),
        ("", secondary) => secondary.to_string(),
        (primary, "") => primary.to_string(),
        (primary, secondary) => format!("{}；{}", primary, secondary),
    }
}

fn set_settings_load_error(
    auth_store: &AuthStore,
    toast_store: &ToastStore,
    mut error_message: Signal<String>,
    err: &AppError,
    prefix: &str,
) {
    if handle_settings_auth_error(auth_store, toast_store, err) {
        error_message.set(settings_auth_expired_message());
    } else {
        error_message.set(format!("{prefix}: {err}"));
    }
}

fn set_settings_action_error(
    auth_store: &AuthStore,
    toast_store: &ToastStore,
    mut error_message: Signal<String>,
    err: &AppError,
    prefix: &str,
) {
    if handle_settings_auth_error(auth_store, toast_store, err) {
        error_message.set(settings_auth_expired_message());
    } else {
        let message = format!("{prefix}: {err}");
        error_message.set(message.clone());
        toast_store.show_error(message);
    }
}

#[component]
pub fn AccountSectionController() -> Element {
    let auth_service = use_auth_service();
    let auth_store = use_auth_store();
    let toast_store = use_toast_store();

    let mut is_logging_out = use_signal(|| false);

    let current_user = auth_store.user();
    let email = current_user
        .as_ref()
        .map(|user| user.email.clone())
        .unwrap_or_else(|| "当前用户".to_string());
    let role = current_user
        .as_ref()
        .map(|user| user.role.clone())
        .unwrap_or_else(|| "user".to_string());
    let created_at = current_user
        .as_ref()
        .map(|user| user.created_at.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "-".to_string());

    let auth_service_for_logout = auth_service.clone();
    let auth_store_for_logout = auth_store.clone();
    let toast_store_for_logout = toast_store.clone();
    let handle_logout = move |_| {
        if is_logging_out() {
            return;
        }

        let auth_service = auth_service_for_logout.clone();
        let auth_store = auth_store_for_logout.clone();
        let toast_store = toast_store_for_logout.clone();
        spawn(async move {
            is_logging_out.set(true);

            match auth_service.logout().await {
                Ok(_) => {
                    toast_store.show_success("已退出登录".to_string());
                }
                Err(err) => {
                    if handle_settings_auth_error(&auth_store, &toast_store, &err) {
                        is_logging_out.set(false);
                        return;
                    }
                    let message = format!("退出登录失败: {}", err);
                    toast_store.show_error(message.clone());
                    eprintln!("{message}");
                }
            }

            is_logging_out.set(false);
        });
    };

    rsx! {
        AccountSettingsSection {
            email,
            role,
            created_at,
            is_logging_out: is_logging_out(),
            on_logout: handle_logout,
        }
    }
}

#[component]
pub fn SecuritySectionController() -> Element {
    let auth_service = use_auth_service();
    let auth_store = use_auth_store();
    let toast_store = use_toast_store();

    let mut current_password = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut error_message = use_signal(String::new);
    let mut success_message = use_signal(String::new);
    let mut is_updating_password = use_signal(|| false);
    let current_user_is_admin = auth_store
        .user()
        .as_ref()
        .is_some_and(|user| user.role.eq_ignore_ascii_case("admin"));
    let min_password_length = if current_user_is_admin { 12 } else { 6 };
    let helper_text = if current_user_is_admin {
        "管理员密码至少需要 12 个字符，建议包含大小写字母、数字与符号。".to_string()
    } else {
        "新密码长度需在 6 到 100 个字符之间。".to_string()
    };

    let auth_service_for_password = auth_service.clone();
    let auth_store_for_password = auth_store.clone();
    let toast_store_for_password = toast_store.clone();
    let handle_change_password = move |_| {
        if is_updating_password() {
            return;
        }

        let current = current_password().trim().to_string();
        let next = new_password().trim().to_string();
        let confirm = confirm_password().trim().to_string();

        error_message.set(String::new());
        success_message.set(String::new());

        if current.is_empty() {
            error_message.set("请输入当前密码".to_string());
            return;
        }

        if next.is_empty() {
            error_message.set("请输入新密码".to_string());
            return;
        }

        if !(min_password_length..=100).contains(&next.len()) {
            error_message.set(format!(
                "新密码长度需在 {} 到 100 个字符之间",
                min_password_length
            ));
            return;
        }

        if next != confirm {
            error_message.set("两次输入的新密码不一致".to_string());
            return;
        }

        let auth_service = auth_service_for_password.clone();
        let auth_store = auth_store_for_password.clone();
        let toast_store = toast_store_for_password.clone();
        spawn(async move {
            is_updating_password.set(true);

            let req = UpdateProfileRequest {
                current_password: current,
                new_password: Some(next),
            };

            match auth_service.change_password(req).await {
                Ok(_) => {
                    current_password.set(String::new());
                    new_password.set(String::new());
                    confirm_password.set(String::new());
                    let message = "密码已更新，请重新登录".to_string();
                    success_message.set(message.clone());
                    toast_store.show_success(message);
                }
                Err(err) => {
                    set_settings_action_error(
                        &auth_store,
                        &toast_store,
                        error_message,
                        &err,
                        "修改密码失败",
                    );
                }
            }

            is_updating_password.set(false);
        });
    };

    rsx! {
        SecuritySettingsSection {
            current_password,
            new_password,
            confirm_password,
            error_message: error_message(),
            success_message: success_message(),
            helper_text,
            is_submitting: is_updating_password(),
            on_submit: handle_change_password,
        }
    }
}

#[component]
pub fn SystemSectionController() -> Element {
    let admin_service = use_admin_service();
    let auth_store = use_auth_store();
    let toast_store = use_toast_store();

    let mut health = use_signal(|| None::<HealthStatus>);
    let mut stats = use_signal(|| None::<SystemStats>);
    let mut error_message = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut reload_tick = use_signal(|| 0_u64);

    let _load_system_status = use_resource({
        let admin_service = admin_service.clone();
        let auth_store = auth_store.clone();
        let toast_store = toast_store.clone();
        move || {
            let _ = reload_tick();
            let admin_service = admin_service.clone();
            let auth_store = auth_store.clone();
            let toast_store = toast_store.clone();
            async move {
                is_loading.set(true);
                error_message.set(String::new());

                let health_result = admin_service.get_health_status().await;
                let stats_result = admin_service.get_system_stats().await;
                let mut errors = Vec::new();

                if let Err(err) = &health_result
                    && handle_settings_auth_error(&auth_store, &toast_store, err)
                {
                    error_message.set(settings_auth_expired_message());
                    is_loading.set(false);
                    return;
                }

                if let Err(err) = &stats_result
                    && handle_settings_auth_error(&auth_store, &toast_store, err)
                {
                    error_message.set(settings_auth_expired_message());
                    is_loading.set(false);
                    return;
                }

                match health_result {
                    Ok(next_health) => health.set(Some(next_health)),
                    Err(err) => errors.push(format!("健康检查接口异常: {}", err)),
                }

                match stats_result {
                    Ok(next_stats) => stats.set(Some(next_stats)),
                    Err(err) => errors.push(format!("统计接口异常: {}", err)),
                }

                if !errors.is_empty() {
                    error_message.set(errors.join("；"));
                }

                is_loading.set(false);
            }
        }
    });

    let handle_refresh = move |_| {
        if is_loading() {
            return;
        }
        reload_tick.set(reload_tick().wrapping_add(1));
    };

    rsx! {
        SystemStatusSection {
            health: health(),
            stats: stats(),
            error_message: error_message(),
            is_loading: is_loading(),
            on_refresh: handle_refresh,
        }
    }
}

#[component]
pub fn MaintenanceSectionController() -> Element {
    let admin_service = use_admin_service();
    let auth_store = use_auth_store();
    let toast_store = use_toast_store();

    let mut error_message = use_signal(String::new);
    let mut backup_list_error_message = use_signal(String::new);
    let mut restore_status_error_message = use_signal(String::new);
    let mut success_message = use_signal(String::new);
    let mut last_backup = use_signal(|| None::<BackupResponse>);
    let mut backup_files = use_signal(Vec::<BackupFileSummary>::new);
    let mut restore_status = use_signal(|| None::<BackupRestoreStatusResponse>);
    let mut last_deleted_cleanup_count = use_signal(|| None::<usize>);
    let mut last_expired_cleanup_count = use_signal(|| None::<i64>);
    let mut is_cleaning_deleted = use_signal(|| false);
    let mut is_cleaning_expired = use_signal(|| false);
    let mut is_backing_up = use_signal(|| false);
    let mut deleting_backup_filename = use_signal(|| None::<String>);
    let mut processing_restore_filename = use_signal(|| None::<String>);
    let mut is_loading_backups = use_signal(|| false);
    let mut is_loading_restore_status = use_signal(|| false);
    let mut reload_backups_tick = use_signal(|| 0_u64);
    let mut reload_restore_status_tick = use_signal(|| 0_u64);
    let mut pending_action = use_signal(|| None::<PendingMaintenanceAction>);

    let _load_backups = use_resource({
        let admin_service = admin_service.clone();
        let auth_store = auth_store.clone();
        let toast_store = toast_store.clone();
        move || {
            let _ = reload_backups_tick();
            let admin_service = admin_service.clone();
            let auth_store = auth_store.clone();
            let toast_store = toast_store.clone();
            async move {
                is_loading_backups.set(true);
                backup_list_error_message.set(String::new());

                match admin_service.get_backups().await {
                    Ok(result) => {
                        let latest_backup = result.first().map(|backup| BackupResponse {
                            filename: backup.filename.clone(),
                            created_at: backup.created_at,
                            semantics: backup.semantics.clone(),
                        });
                        backup_files.set(result);
                        last_backup.set(latest_backup);
                    }
                    Err(err) => {
                        set_settings_load_error(
                            &auth_store,
                            &toast_store,
                            backup_list_error_message,
                            &err,
                            "加载备份列表失败",
                        );
                    }
                }

                is_loading_backups.set(false);
            }
        }
    });

    let _load_restore_status = use_resource({
        let admin_service = admin_service.clone();
        let auth_store = auth_store.clone();
        let toast_store = toast_store.clone();
        move || {
            let _ = reload_restore_status_tick();
            let admin_service = admin_service.clone();
            let auth_store = auth_store.clone();
            let toast_store = toast_store.clone();
            async move {
                is_loading_restore_status.set(true);
                restore_status_error_message.set(String::new());

                match admin_service.get_backup_restore_status().await {
                    Ok(status) => restore_status.set(Some(status)),
                    Err(err) => {
                        set_settings_load_error(
                            &auth_store,
                            &toast_store,
                            restore_status_error_message,
                            &err,
                            "加载恢复状态失败",
                        );
                    }
                }

                is_loading_restore_status.set(false);
            }
        }
    });

    let admin_service_for_deleted_cleanup = admin_service.clone();
    let auth_store_for_deleted_cleanup = auth_store.clone();
    let toast_store_for_deleted_cleanup = toast_store.clone();
    let run_cleanup_deleted = move || {
        let admin_service = admin_service_for_deleted_cleanup.clone();
        let auth_store = auth_store_for_deleted_cleanup.clone();
        let toast_store = toast_store_for_deleted_cleanup.clone();
        spawn(async move {
            is_cleaning_deleted.set(true);
            error_message.set(String::new());
            success_message.set(String::new());

            match admin_service.cleanup_deleted_files().await {
                Ok(removed) => {
                    let count = removed.len();
                    last_deleted_cleanup_count.set(Some(count));
                    let message = if count == 0 {
                        "当前没有可清理的已删除文件".to_string()
                    } else {
                        format!("已永久清理 {} 个已删除文件", count)
                    };
                    success_message.set(message.clone());
                    toast_store.show_success(message);
                }
                Err(err) => {
                    set_settings_action_error(
                        &auth_store,
                        &toast_store,
                        error_message,
                        &err,
                        "清理已删除文件失败",
                    );
                }
            }

            is_cleaning_deleted.set(false);
        });
    };

    let handle_cleanup_deleted = move |_| {
        if is_cleaning_deleted()
            || is_cleaning_expired()
            || is_backing_up()
            || deleting_backup_filename().is_some()
            || processing_restore_filename().is_some()
        {
            return;
        }
        pending_action.set(Some(PendingMaintenanceAction {
            action: MaintenanceAction::CleanupDeleted,
            plan: maintenance_confirmation_plan(MaintenanceAction::CleanupDeleted),
        }));
    };

    let admin_service_for_expired_cleanup = admin_service.clone();
    let auth_store_for_expired_cleanup = auth_store.clone();
    let toast_store_for_expired_cleanup = toast_store.clone();
    let run_cleanup_expired = move || {
        let admin_service = admin_service_for_expired_cleanup.clone();
        let auth_store = auth_store_for_expired_cleanup.clone();
        let toast_store = toast_store_for_expired_cleanup.clone();
        spawn(async move {
            is_cleaning_expired.set(true);
            error_message.set(String::new());
            success_message.set(String::new());

            match admin_service.cleanup_expired_images().await {
                Ok(affected) => {
                    last_expired_cleanup_count.set(Some(affected));
                    let message = if affected <= 0 {
                        "当前没有需要处理的过期图片".to_string()
                    } else {
                        format!("已处理 {} 张过期图片", affected)
                    };
                    success_message.set(message.clone());
                    toast_store.show_success(message);
                }
                Err(err) => {
                    set_settings_action_error(
                        &auth_store,
                        &toast_store,
                        error_message,
                        &err,
                        "处理过期图片失败",
                    );
                }
            }

            is_cleaning_expired.set(false);
        });
    };

    let handle_cleanup_expired = move |_| {
        if is_cleaning_deleted()
            || is_cleaning_expired()
            || is_backing_up()
            || deleting_backup_filename().is_some()
            || processing_restore_filename().is_some()
        {
            return;
        }
        pending_action.set(Some(PendingMaintenanceAction {
            action: MaintenanceAction::CleanupExpired,
            plan: maintenance_confirmation_plan(MaintenanceAction::CleanupExpired),
        }));
    };

    let admin_service_for_backup = admin_service.clone();
    let auth_store_for_backup = auth_store.clone();
    let toast_store_for_backup = toast_store.clone();
    let handle_backup_database = move |_| {
        if is_cleaning_deleted()
            || is_cleaning_expired()
            || is_backing_up()
            || deleting_backup_filename().is_some()
            || processing_restore_filename().is_some()
        {
            return;
        }

        let admin_service = admin_service_for_backup.clone();
        let auth_store = auth_store_for_backup.clone();
        let toast_store = toast_store_for_backup.clone();
        spawn(async move {
            is_backing_up.set(true);
            error_message.set(String::new());
            success_message.set(String::new());

            match admin_service.backup_database().await {
                Ok(backup) => {
                    let filename = backup.filename.clone();
                    last_backup.set(Some(backup));
                    reload_backups_tick.set(reload_backups_tick().wrapping_add(1));
                    let message = format!("已生成数据库备份: {}", filename);
                    success_message.set(message.clone());
                    toast_store.show_success(message);
                }
                Err(err) => {
                    set_settings_action_error(
                        &auth_store,
                        &toast_store,
                        error_message,
                        &err,
                        "数据库备份失败",
                    );
                }
            }

            is_backing_up.set(false);
        });
    };

    let admin_service_for_delete_backup = admin_service.clone();
    let auth_store_for_delete_backup = auth_store.clone();
    let toast_store_for_delete_backup = toast_store.clone();
    let run_delete_backup = move |filename: String| {
        let admin_service = admin_service_for_delete_backup.clone();
        let auth_store = auth_store_for_delete_backup.clone();
        let toast_store = toast_store_for_delete_backup.clone();
        spawn(async move {
            deleting_backup_filename.set(Some(filename.clone()));
            error_message.set(String::new());
            success_message.set(String::new());

            match admin_service.delete_backup(&filename).await {
                Ok(_) => {
                    let message = format!("已删除备份文件: {}", filename);
                    success_message.set(message.clone());
                    toast_store.show_success(message);
                    reload_backups_tick.set(reload_backups_tick().wrapping_add(1));
                }
                Err(err) => {
                    set_settings_action_error(
                        &auth_store,
                        &toast_store,
                        error_message,
                        &err,
                        "删除备份文件失败",
                    );
                }
            }

            deleting_backup_filename.set(None);
        });
    };

    let handle_delete_backup = move |filename: String| {
        if is_cleaning_deleted()
            || is_cleaning_expired()
            || is_backing_up()
            || deleting_backup_filename().is_some()
            || processing_restore_filename().is_some()
        {
            return;
        }

        let action = MaintenanceAction::DeleteBackup(filename.clone());
        pending_action.set(Some(PendingMaintenanceAction {
            action: action.clone(),
            plan: maintenance_confirmation_plan(action),
        }));
    };

    let handle_refresh_backups = move |_| {
        if is_cleaning_deleted()
            || is_cleaning_expired()
            || is_backing_up()
            || deleting_backup_filename().is_some()
            || processing_restore_filename().is_some()
            || is_loading_backups()
        {
            return;
        }

        reload_backups_tick.set(reload_backups_tick().wrapping_add(1));
    };

    let handle_refresh_restore_status = move |_| {
        if is_cleaning_deleted()
            || is_cleaning_expired()
            || is_backing_up()
            || deleting_backup_filename().is_some()
            || processing_restore_filename().is_some()
            || is_loading_restore_status()
        {
            return;
        }

        reload_restore_status_tick.set(reload_restore_status_tick().wrapping_add(1));
    };

    let admin_service_for_schedule_restore = admin_service.clone();
    let auth_store_for_schedule_restore = auth_store.clone();
    let toast_store_for_schedule_restore = toast_store.clone();
    let run_schedule_restore = move |filename: String| {
        let admin_service = admin_service_for_schedule_restore.clone();
        let auth_store = auth_store_for_schedule_restore.clone();
        let toast_store = toast_store_for_schedule_restore.clone();
        spawn(async move {
            processing_restore_filename.set(Some(filename.clone()));
            error_message.set(String::new());
            success_message.set(String::new());

            match admin_service.schedule_backup_restore(&filename).await {
                Ok(response) => {
                    reload_restore_status_tick.set(reload_restore_status_tick().wrapping_add(1));
                    let message = format!(
                        "恢复计划已写入，请立即重启服务：{}",
                        response.pending.filename
                    );
                    success_message.set(message.clone());
                    toast_store.show_success(message);
                }
                Err(err) => {
                    set_settings_action_error(
                        &auth_store,
                        &toast_store,
                        error_message,
                        &err,
                        "写入恢复计划失败",
                    );
                }
            }

            processing_restore_filename.set(None);
        });
    };

    let admin_service_for_restore_precheck = admin_service.clone();
    let auth_store_for_restore_precheck = auth_store.clone();
    let toast_store_for_restore_precheck = toast_store.clone();
    let handle_restore_backup = move |backup: BackupFileSummary| {
        if is_cleaning_deleted()
            || is_cleaning_expired()
            || is_backing_up()
            || deleting_backup_filename().is_some()
            || processing_restore_filename().is_some()
        {
            return;
        }

        let filename = backup.filename.clone();
        if !backup_supports_restore(&backup.semantics) {
            let message = format!(
                "当前页面不支持恢复这类备份：{}（{}）。按 1.0 范围，这类备份只支持下载或运维侧恢复。",
                filename,
                backup_kind_label(&backup.semantics)
            );
            error_message.set(message.clone());
            toast_store_for_restore_precheck.show_error(message);
            return;
        }

        if restore_status()
            .as_ref()
            .is_some_and(|status| status.pending.is_some())
        {
            let message = "已有待执行的恢复计划，请先重启服务完成当前计划。".to_string();
            error_message.set(message.clone());
            toast_store_for_restore_precheck.show_error(message);
            return;
        }

        let admin_service = admin_service_for_restore_precheck.clone();
        let auth_store = auth_store_for_restore_precheck.clone();
        let toast_store = toast_store_for_restore_precheck.clone();
        spawn(async move {
            processing_restore_filename.set(Some(filename.clone()));
            error_message.set(String::new());
            success_message.set(String::new());

            match admin_service.precheck_backup_restore(&filename).await {
                Ok(precheck) => {
                    if precheck.eligible {
                        pending_action.set(Some(PendingMaintenanceAction {
                            action: MaintenanceAction::RestoreBackup(filename),
                            plan: restore_confirmation_plan(&precheck),
                        }));
                    } else {
                        let message = restore_precheck_error_message(&precheck);
                        error_message.set(message.clone());
                        toast_store.show_error(message);
                    }
                }
                Err(err) => {
                    set_settings_action_error(
                        &auth_store,
                        &toast_store,
                        error_message,
                        &err,
                        "恢复预检失败",
                    );
                }
            }

            processing_restore_filename.set(None);
        });
    };

    let backup_downloads = backup_files()
        .into_iter()
        .map(|backup| {
            let download_url = admin_service.backup_download_url(&backup.filename);
            (backup, download_url)
        })
        .collect::<Vec<_>>();

    rsx! {
        MaintenanceSettingsSection {
            error_message: merge_messages(
                &merge_messages(&error_message(), &backup_list_error_message()),
                &restore_status_error_message(),
            ),
            success_message: success_message(),
            last_backup: last_backup(),
            backup_files: backup_downloads,
            restore_status: restore_status(),
            last_deleted_cleanup_count: last_deleted_cleanup_count(),
            last_expired_cleanup_count: last_expired_cleanup_count(),
            is_cleaning_deleted: is_cleaning_deleted(),
            is_cleaning_expired: is_cleaning_expired(),
            is_backing_up: is_backing_up(),
            deleting_backup_filename: deleting_backup_filename(),
            processing_restore_filename: processing_restore_filename(),
            is_loading_backups: is_loading_backups(),
            is_loading_restore_status: is_loading_restore_status(),
            on_cleanup_deleted: handle_cleanup_deleted,
            on_cleanup_expired: handle_cleanup_expired,
            on_backup: handle_backup_database,
            on_refresh_backups: handle_refresh_backups,
            on_refresh_restore_status: handle_refresh_restore_status,
            on_delete_backup: handle_delete_backup,
            on_restore_backup: handle_restore_backup,
        }

        if let Some(pending) = pending_action() {
            ConfirmationModal {
                title: pending.plan.title.clone(),
                summary: pending.plan.summary.clone(),
                consequences: pending.plan.consequences.clone(),
                confirm_label: pending.plan.confirm_label.clone(),
                cancel_label: pending.plan.cancel_label.clone(),
                tone: pending.plan.tone,
                confirm_phrase: pending.plan.confirm_phrase.clone(),
                confirm_hint: pending.plan.confirm_hint.clone(),
                is_submitting: is_cleaning_deleted()
                    || is_cleaning_expired()
                    || is_backing_up()
                    || deleting_backup_filename().is_some()
                    || processing_restore_filename().is_some(),
                on_close: move |_| pending_action.set(None),
                on_confirm: move |_| {
                    let action = pending.action.clone();
                    pending_action.set(None);
                    match action {
                        MaintenanceAction::CleanupDeleted => run_cleanup_deleted(),
                        MaintenanceAction::CleanupExpired => run_cleanup_expired(),
                        MaintenanceAction::DeleteBackup(filename) => run_delete_backup(filename),
                        MaintenanceAction::RestoreBackup(filename) => run_schedule_restore(filename),
                    }
                },
            }
        }
    }
}

#[component]
pub fn UsersSectionController() -> Element {
    let admin_service = use_admin_service();
    let auth_store = use_auth_store();
    let toast_store = use_toast_store();

    let mut users = use_signal(Vec::<AdminUserSummary>::new);
    let mut role_drafts = use_signal(HashMap::<String, String>::new);
    let mut error_message = use_signal(String::new);
    let mut success_message = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut reload_tick = use_signal(|| 0_u64);
    let mut updating_user_id = use_signal(|| None::<String>);
    let mut pending_role_change = use_signal(|| None::<PendingUserRoleChange>);

    let _load_users = use_resource({
        let admin_service = admin_service.clone();
        let auth_store = auth_store.clone();
        let toast_store = toast_store.clone();
        move || {
            let _ = reload_tick();
            let admin_service = admin_service.clone();
            let auth_store = auth_store.clone();
            let toast_store = toast_store.clone();
            async move {
                is_loading.set(true);
                error_message.set(String::new());

                match admin_service.get_users().await {
                    Ok(result) => {
                        let next_role_map = result
                            .iter()
                            .map(|user| (user.id.clone(), user.role.clone()))
                            .collect();
                        users.set(result);
                        role_drafts.set(next_role_map);
                    }
                    Err(err) => {
                        set_settings_load_error(
                            &auth_store,
                            &toast_store,
                            error_message,
                            &err,
                            "加载用户列表失败",
                        );
                    }
                }

                is_loading.set(false);
            }
        }
    });

    let handle_refresh = move |_| {
        if is_loading() || updating_user_id().is_some() {
            return;
        }
        reload_tick.set(reload_tick().wrapping_add(1));
    };

    let toast_store_for_user_role = toast_store.clone();
    let auth_store_for_confirm_role = auth_store.clone();
    let admin_service_for_confirm_role = admin_service.clone();
    let toast_store_for_confirm_role = toast_store.clone();
    let handle_save_user_role = move |user_id: String| {
        if updating_user_id().is_some() {
            return;
        }

        let current_users = users();
        let Some(current_user) = current_users.iter().find(|user| user.id == user_id) else {
            let message = "未找到要更新的用户".to_string();
            error_message.set(message.clone());
            toast_store_for_user_role.show_error(message);
            return;
        };

        let next_role = role_drafts()
            .get(&user_id)
            .cloned()
            .unwrap_or_else(|| current_user.role.clone());

        if next_role == current_user.role {
            let message = format!("{} 的角色未发生变化", current_user.email);
            success_message.set(message.clone());
            toast_store_for_user_role.show_info(message);
            return;
        }

        let email = current_user.email.clone();
        pending_role_change.set(Some(PendingUserRoleChange {
            user_id,
            email: email.clone(),
            next_role: next_role.clone(),
            plan: role_change_confirmation_plan(&email, &current_user.role, &next_role),
        }));
    };

    rsx! {
        UsersSettingsSection {
            users: users(),
            role_drafts,
            error_message: error_message(),
            success_message: success_message(),
            is_loading: is_loading(),
            updating_user_id: updating_user_id(),
            on_refresh: handle_refresh,
            on_save_role: handle_save_user_role,
        }

        if let Some(pending) = pending_role_change() {
            ConfirmationModal {
                title: pending.plan.title.clone(),
                summary: pending.plan.summary.clone(),
                consequences: pending.plan.consequences.clone(),
                confirm_label: pending.plan.confirm_label.clone(),
                cancel_label: pending.plan.cancel_label.clone(),
                tone: pending.plan.tone,
                confirm_phrase: pending.plan.confirm_phrase.clone(),
                confirm_hint: pending.plan.confirm_hint.clone(),
                is_submitting: updating_user_id().is_some(),
                on_close: move |_| pending_role_change.set(None),
                on_confirm: move |_| {
                    let user_id = pending.user_id.clone();
                    let email = pending.email.clone();
                    let next_role = pending.next_role.clone();
                    pending_role_change.set(None);

                    let admin_service = admin_service_for_confirm_role.clone();
                    let auth_store = auth_store_for_confirm_role.clone();
                    let toast_store = toast_store_for_confirm_role.clone();
                    spawn(async move {
                        updating_user_id.set(Some(user_id.clone()));
                        error_message.set(String::new());
                        success_message.set(String::new());

                        match admin_service
                            .update_user_role(&user_id, next_role.clone())
                            .await
                        {
                            Ok(_) => {
                                let mut next_users = users();
                                if let Some(user) = next_users.iter_mut().find(|user| user.id == user_id) {
                                    user.role = next_role.clone();
                                }
                                users.set(next_users);

                                let mut drafts = role_drafts();
                                drafts.insert(user_id.clone(), next_role.clone());
                                role_drafts.set(drafts);

                                let message = format!("已将 {} 的角色更新为 {}", email, next_role);
                                success_message.set(message.clone());
                                toast_store.show_success(message);
                            }
                            Err(err) => {
                                set_settings_action_error(
                                    &auth_store,
                                    &toast_store,
                                    error_message,
                                    &err,
                                    "更新用户角色失败",
                                );
                            }
                        }

                        updating_user_id.set(None);
                    });
                },
            }
        }
    }
}

#[component]
pub fn AuditSectionController() -> Element {
    let admin_service = use_admin_service();
    let auth_store = use_auth_store();
    let toast_store = use_toast_store();

    let mut logs = use_signal(Vec::<AuditLog>::new);
    let mut total = use_signal(|| 0_i64);
    let mut error_message = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut reload_tick = use_signal(|| 0_u64);
    let mut current_page = use_signal(|| 1_i32);
    let mut page_size = use_signal(|| 20_i32);

    let _load_audit_logs = use_resource({
        let admin_service = admin_service.clone();
        let auth_store = auth_store.clone();
        let toast_store = toast_store.clone();
        move || {
            let _ = reload_tick();
            let page = current_page().max(1);
            let size = page_size().clamp(1, 100);
            let admin_service = admin_service.clone();
            let auth_store = auth_store.clone();
            let toast_store = toast_store.clone();
            async move {
                is_loading.set(true);
                error_message.set(String::new());

                let params = PaginationParams {
                    page: Some(page),
                    page_size: Some(size),
                    tag: None,
                };

                match admin_service.get_audit_logs(params).await {
                    Ok(result) => {
                        if result.data.is_empty() && page > 1 && result.total > 0 {
                            current_page.set(page - 1);
                            is_loading.set(false);
                            return;
                        }

                        let normalized_page = result.page.max(1);
                        if normalized_page != page {
                            current_page.set(normalized_page);
                        }
                        if result.page_size != size {
                            page_size.set(result.page_size.clamp(1, 100));
                        }

                        logs.set(result.data);
                        total.set(result.total);
                    }
                    Err(err) => {
                        set_settings_load_error(
                            &auth_store,
                            &toast_store,
                            error_message,
                            &err,
                            "加载审计日志失败",
                        );
                    }
                }

                is_loading.set(false);
            }
        }
    });

    let handle_refresh = move |_| {
        if is_loading() {
            return;
        }
        reload_tick.set(reload_tick().wrapping_add(1));
    };

    let handle_prev_page = move |_| {
        if is_loading() || current_page() <= 1 {
            return;
        }
        current_page.set(current_page() - 1);
    };

    let handle_next_page = move |_| {
        let has_next = (current_page() as i64 * page_size() as i64) < total();
        if is_loading() || !has_next {
            return;
        }
        current_page.set(current_page() + 1);
    };

    let handle_page_size_change = move |value: String| {
        if let Ok(size) = value.parse::<i32>() {
            let normalized_size = size.clamp(1, 100);
            if normalized_size != page_size() {
                page_size.set(normalized_size);
                current_page.set(1);
            }
        }
    };

    let has_next = (current_page() as i64 * page_size() as i64) < total();

    rsx! {
        AuditSettingsSection {
            logs: logs(),
            total: total(),
            current_page: current_page(),
            page_size: page_size(),
            has_next,
            error_message: error_message(),
            is_loading: is_loading(),
            on_refresh: handle_refresh,
            on_prev_page: handle_prev_page,
            on_next_page: handle_next_page,
            on_page_size_change: handle_page_size_change,
        }
    }
}

#[component]
pub fn AdvancedSectionController(
    #[props(default)] on_site_name_updated: EventHandler<String>,
) -> Element {
    let admin_service = use_admin_service();
    let auth_store = use_auth_store();
    let toast_store = use_toast_store();

    let mut settings = use_signal(Vec::<Setting>::new);
    let mut setting_drafts = use_signal(HashMap::<String, String>::new);
    let mut error_message = use_signal(String::new);
    let mut success_message = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut reload_tick = use_signal(|| 0_u64);
    let mut saving_key = use_signal(|| None::<String>);
    let mut pending_setting_change = use_signal(|| None::<PendingSettingChange>);

    let _load_raw_settings = use_resource({
        let admin_service = admin_service.clone();
        let auth_store = auth_store.clone();
        let toast_store = toast_store.clone();
        move || {
            let _ = reload_tick();
            let admin_service = admin_service.clone();
            let auth_store = auth_store.clone();
            let toast_store = toast_store.clone();
            async move {
                is_loading.set(true);
                error_message.set(String::new());

                match admin_service.get_raw_settings().await {
                    Ok(result) => {
                        let draft_map = result
                            .iter()
                            .map(|setting| (setting.key.clone(), setting.value.clone()))
                            .collect();
                        settings.set(result);
                        setting_drafts.set(draft_map);
                    }
                    Err(err) => {
                        set_settings_load_error(
                            &auth_store,
                            &toast_store,
                            error_message,
                            &err,
                            "加载原始设置失败",
                        );
                    }
                }

                is_loading.set(false);
            }
        }
    });

    let handle_refresh = move |_| {
        if is_loading() || saving_key().is_some() {
            return;
        }
        reload_tick.set(reload_tick().wrapping_add(1));
    };

    let admin_service_for_raw_setting = admin_service.clone();
    let auth_store_for_raw_setting = auth_store.clone();
    let toast_store_for_raw_setting = toast_store.clone();
    let on_site_name_updated_for_raw_setting = on_site_name_updated;
    let admin_service_for_confirm_setting = admin_service.clone();
    let auth_store_for_confirm_setting = auth_store.clone();
    let toast_store_for_confirm_setting = toast_store.clone();
    let on_site_name_updated_for_confirm_setting = on_site_name_updated;
    let handle_save_setting = move |key: String| {
        if saving_key().is_some() {
            return;
        }

        let current_settings = settings();
        let Some(current_setting) = current_settings.iter().find(|setting| setting.key == key)
        else {
            let message = "未找到要更新的设置项".to_string();
            error_message.set(message.clone());
            toast_store_for_raw_setting.show_error(message);
            return;
        };

        if !current_setting.editable {
            let message = format!("设置项 {} 受保护，不能通过高级设置直接修改", key);
            error_message.set(message.clone());
            toast_store_for_raw_setting.show_error(message);
            return;
        }

        let next_value = setting_drafts()
            .get(&key)
            .cloned()
            .unwrap_or_else(|| current_setting.value.clone());

        if next_value == current_setting.value {
            let message = format!("键值 {} 未发生变化", key);
            success_message.set(message.clone());
            toast_store_for_raw_setting.show_info(message);
            return;
        }

        if current_setting.requires_confirmation {
            let plan = advanced_setting_confirmation_plan(&key, &next_value);
            pending_setting_change.set(Some(PendingSettingChange {
                key,
                next_value,
                plan,
            }));
            return;
        }

        let admin_service = admin_service_for_raw_setting.clone();
        let auth_store = auth_store_for_raw_setting.clone();
        let toast_store = toast_store_for_raw_setting.clone();
        let on_site_name_updated = on_site_name_updated_for_raw_setting;
        spawn(async move {
            saving_key.set(Some(key.clone()));
            error_message.set(String::new());
            success_message.set(String::new());

            match admin_service.update_setting(&key, next_value.clone()).await {
                Ok(_) => {
                    let message = format!("已更新原始设置 {}", key);
                    success_message.set(message.clone());
                    toast_store.show_success(message);
                    if setting_requires_restart(&key) {
                        toast_store.show_info("存储相关设置将在服务重启后生效".to_string());
                    }
                    if key == "site_name" {
                        on_site_name_updated.call(next_value.trim().to_string());
                    }
                    reload_tick.set(reload_tick().wrapping_add(1));
                }
                Err(err) => {
                    set_settings_action_error(
                        &auth_store,
                        &toast_store,
                        error_message,
                        &err,
                        "更新原始设置失败",
                    );
                }
            }

            saving_key.set(None);
        });
    };

    rsx! {
        AdvancedSettingsSection {
            settings: settings(),
            setting_drafts,
            error_message: error_message(),
            success_message: success_message(),
            is_loading: is_loading(),
            saving_key: saving_key(),
            on_refresh: handle_refresh,
            on_save_setting: handle_save_setting,
        }

        if let Some(pending) = pending_setting_change() {
            ConfirmationModal {
                title: pending.plan.title.clone(),
                summary: pending.plan.summary.clone(),
                consequences: pending.plan.consequences.clone(),
                confirm_label: pending.plan.confirm_label.clone(),
                cancel_label: pending.plan.cancel_label.clone(),
                tone: pending.plan.tone,
                confirm_phrase: pending.plan.confirm_phrase.clone(),
                confirm_hint: pending.plan.confirm_hint.clone(),
                is_submitting: saving_key().is_some(),
                on_close: move |_| pending_setting_change.set(None),
                on_confirm: move |_| {
                    let key = pending.key.clone();
                    let next_value = pending.next_value.clone();
                    pending_setting_change.set(None);

                    let admin_service = admin_service_for_confirm_setting.clone();
                    let auth_store = auth_store_for_confirm_setting.clone();
                    let toast_store = toast_store_for_confirm_setting.clone();
                    let on_site_name_updated = on_site_name_updated_for_confirm_setting;
                    spawn(async move {
                        saving_key.set(Some(key.clone()));
                        error_message.set(String::new());
                        success_message.set(String::new());

                        match admin_service.update_setting(&key, next_value.clone()).await {
                            Ok(_) => {
                                let message = format!("已更新原始设置 {}", key);
                                success_message.set(message.clone());
                                toast_store.show_success(message);
                                if setting_requires_restart(&key) {
                                    toast_store.show_info("存储相关设置将在服务重启后生效".to_string());
                                }
                                if key == "site_name" {
                                    on_site_name_updated.call(next_value.trim().to_string());
                                }
                                reload_tick.set(reload_tick().wrapping_add(1));
                            }
                            Err(err) => {
                                set_settings_action_error(
                                    &auth_store,
                                    &toast_store,
                                    error_message,
                                    &err,
                                    "更新原始设置失败",
                                );
                            }
                        }

                        saving_key.set(None);
                    });
                },
            }
        }
    }
}
