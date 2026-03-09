mod state;
mod view;

use crate::app_context::{
    use_admin_service, use_auth_service, use_auth_store, use_settings_service, use_toast_store,
};
use crate::types::api::{
    AdminUserSummary, AuditLog, BackupResponse, HealthStatus, PaginationParams, Setting,
    SystemStats, UpdateProfileRequest,
};
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use std::collections::HashMap;

use state::SettingsFormState;
use view::{
    ADMIN_SETTINGS_SECTIONS, AccountSettingsSection, AdvancedSettingsSection, AuditSettingsSection,
    MaintenanceSettingsSection, SecuritySettingsSection, SettingsSection, SystemStatusSection,
    USER_SETTINGS_SECTIONS, UsersSettingsSection, render_settings_fields,
};

#[cfg(target_arch = "wasm32")]
fn confirm_action(message: &str) -> bool {
    web_sys::window()
        .and_then(|window| window.confirm_with_message(message).ok())
        .unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
fn confirm_action(_message: &str) -> bool {
    true
}

fn advanced_setting_confirm_message(key: &str) -> &'static str {
    match key {
        "storage_backend" => "确认修改 storage_backend 吗？这会直接影响后端选用的存储方式。",
        "local_storage_path" => "确认修改 local_storage_path 吗？这可能改变文件的实际写入目录。",
        "s3_endpoint" | "s3_region" | "s3_bucket" | "s3_prefix" => {
            "确认修改这项 S3 运行时设置吗？错误配置会影响对象存储读写。"
        }
        "s3_force_path_style" => {
            "确认修改 s3_force_path_style 吗？这可能影响 S3/MinIO 的访问方式。"
        }
        _ => "确认保存这项高级设置吗？",
    }
}

const SETTINGS_LOAD_RETRY_DELAYS_MS: [u32; 3] = [0, 500, 1500];

#[component]
pub fn SettingsPage(
    is_admin: bool,
    #[props(default)] on_site_name_updated: EventHandler<String>,
) -> Element {
    let settings_service = use_settings_service();
    let auth_service = use_auth_service();
    let auth_store = use_auth_store();
    let admin_service = use_admin_service();
    let toast_store = use_toast_store();

    let mut is_loading = use_signal(|| true);
    let mut is_saving = use_signal(|| false);
    let mut error_message = use_signal(String::new);
    let mut reload_tick = use_signal(|| 0_u64);
    let mut active_section = use_signal(move || {
        if is_admin {
            SettingsSection::General
        } else {
            SettingsSection::Account
        }
    });

    let site_name = use_signal(String::new);
    let storage_backend = use_signal(|| "local".to_string());
    let local_storage_path = use_signal(String::new);
    let s3_endpoint = use_signal(String::new);
    let s3_region = use_signal(String::new);
    let s3_bucket = use_signal(String::new);
    let s3_prefix = use_signal(String::new);
    let s3_access_key = use_signal(String::new);
    let s3_secret_key = use_signal(String::new);
    let s3_secret_key_set = use_signal(|| false);
    let s3_force_path_style = use_signal(|| true);

    let mut current_password = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut security_error_message = use_signal(String::new);
    let mut security_success_message = use_signal(String::new);
    let mut is_updating_password = use_signal(|| false);
    let mut is_logging_out = use_signal(|| false);

    let mut system_health = use_signal(|| None::<HealthStatus>);
    let mut system_stats = use_signal(|| None::<SystemStats>);
    let mut system_error_message = use_signal(String::new);
    let mut is_loading_system = use_signal(|| false);
    let mut system_reload_tick = use_signal(|| 0_u64);
    let mut system_has_loaded = use_signal(|| false);
    let mut system_loaded_tick = use_signal(|| 0_u64);

    let mut maintenance_error_message = use_signal(String::new);
    let mut maintenance_success_message = use_signal(String::new);
    let mut last_backup = use_signal(|| None::<BackupResponse>);
    let mut last_deleted_cleanup_count = use_signal(|| None::<usize>);
    let mut last_expired_cleanup_count = use_signal(|| None::<i64>);
    let mut is_cleaning_deleted = use_signal(|| false);
    let mut is_cleaning_expired = use_signal(|| false);
    let mut is_backing_up = use_signal(|| false);

    let mut users = use_signal(Vec::<AdminUserSummary>::new);
    let mut user_role_drafts = use_signal(HashMap::<String, String>::new);
    let mut users_error_message = use_signal(String::new);
    let mut users_success_message = use_signal(String::new);
    let mut is_loading_users = use_signal(|| false);
    let mut users_reload_tick = use_signal(|| 0_u64);
    let mut updating_user_id = use_signal(|| None::<String>);

    let mut audit_logs = use_signal(Vec::<AuditLog>::new);
    let mut audit_total = use_signal(|| 0_i64);
    let mut audit_error_message = use_signal(String::new);
    let mut is_loading_audit = use_signal(|| false);
    let mut audit_reload_tick = use_signal(|| 0_u64);
    let mut audit_current_page = use_signal(|| 1_i32);
    let mut audit_page_size = use_signal(|| 20_i32);

    let mut raw_settings = use_signal(Vec::<Setting>::new);
    let mut raw_setting_drafts = use_signal(HashMap::<String, String>::new);
    let mut advanced_error_message = use_signal(String::new);
    let mut advanced_success_message = use_signal(String::new);
    let mut is_loading_advanced = use_signal(|| false);
    let mut advanced_reload_tick = use_signal(|| 0_u64);
    let mut saving_setting_key = use_signal(|| None::<String>);

    let form = SettingsFormState {
        site_name,
        storage_backend,
        local_storage_path,
        s3_endpoint,
        s3_region,
        s3_bucket,
        s3_prefix,
        s3_access_key,
        s3_secret_key,
        s3_secret_key_set,
        s3_force_path_style,
    };

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

    let _load_system_status = use_resource({
        let admin_service = admin_service.clone();
        move || {
            let section = active_section();
            let reload = system_reload_tick();
            let has_loaded = system_has_loaded();
            let loaded_tick = system_loaded_tick();
            let admin_service = admin_service.clone();
            async move {
                if !is_admin || section != SettingsSection::System {
                    return;
                }

                if has_loaded && loaded_tick == reload {
                    return;
                }

                is_loading_system.set(true);
                system_error_message.set(String::new());

                let health_result = admin_service.get_health_status().await;
                let stats_result = admin_service.get_system_stats().await;
                let mut errors = Vec::new();

                match health_result {
                    Ok(health) => system_health.set(Some(health)),
                    Err(err) => errors.push(format!("健康检查接口异常: {}", err)),
                }

                match stats_result {
                    Ok(stats) => system_stats.set(Some(stats)),
                    Err(err) => errors.push(format!("统计接口异常: {}", err)),
                }

                if errors.is_empty() {
                    system_has_loaded.set(true);
                    system_loaded_tick.set(reload);
                } else {
                    system_error_message.set(errors.join("；"));
                }

                is_loading_system.set(false);
            }
        }
    });

    let _load_users = use_resource({
        let admin_service = admin_service.clone();
        move || {
            let section = active_section();
            let _reload = users_reload_tick();
            let admin_service = admin_service.clone();
            async move {
                if !is_admin || section != SettingsSection::Users {
                    return;
                }

                is_loading_users.set(true);
                users_error_message.set(String::new());

                match admin_service.get_users().await {
                    Ok(result) => {
                        let role_map = result
                            .iter()
                            .map(|user| (user.id.clone(), user.role.clone()))
                            .collect();
                        users.set(result);
                        user_role_drafts.set(role_map);
                    }
                    Err(err) => {
                        users_error_message.set(format!("加载用户列表失败: {}", err));
                    }
                }

                is_loading_users.set(false);
            }
        }
    });

    let _load_audit_logs = use_resource({
        let admin_service = admin_service.clone();
        move || {
            let section = active_section();
            let _reload = audit_reload_tick();
            let page = audit_current_page().max(1);
            let page_size = audit_page_size().clamp(1, 100);
            let admin_service = admin_service.clone();
            async move {
                if !is_admin || section != SettingsSection::Audit {
                    return;
                }

                is_loading_audit.set(true);
                audit_error_message.set(String::new());

                let params = PaginationParams {
                    page: Some(page),
                    page_size: Some(page_size),
                    category_id: None,
                    tag: None,
                };

                match admin_service.get_audit_logs(params).await {
                    Ok(result) => {
                        if result.data.is_empty() && page > 1 && result.total > 0 {
                            audit_current_page.set(page - 1);
                            is_loading_audit.set(false);
                            return;
                        }

                        let normalized_page = result.page.max(1);
                        if normalized_page != page {
                            audit_current_page.set(normalized_page);
                        }
                        if result.page_size != page_size {
                            audit_page_size.set(result.page_size.clamp(1, 100));
                        }

                        audit_logs.set(result.data);
                        audit_total.set(result.total);
                    }
                    Err(err) => {
                        audit_error_message.set(format!("加载审计日志失败: {}", err));
                    }
                }

                is_loading_audit.set(false);
            }
        }
    });

    let _load_raw_settings = use_resource({
        let admin_service = admin_service.clone();
        move || {
            let section = active_section();
            let _reload = advanced_reload_tick();
            let admin_service = admin_service.clone();
            async move {
                if !is_admin || section != SettingsSection::Advanced {
                    return;
                }

                is_loading_advanced.set(true);
                advanced_error_message.set(String::new());

                match admin_service.get_raw_settings().await {
                    Ok(result) => {
                        let draft_map = result
                            .iter()
                            .map(|setting| (setting.key.clone(), setting.value.clone()))
                            .collect();
                        raw_settings.set(result);
                        raw_setting_drafts.set(draft_map);
                    }
                    Err(err) => {
                        advanced_error_message.set(format!("加载原始设置失败: {}", err));
                    }
                }

                is_loading_advanced.set(false);
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
                    if err.should_redirect_login() {
                        auth_store.logout();
                    }
                    let message = format!("退出登录失败: {}", err);
                    toast_store.show_error(message.clone());
                    eprintln!("{message}");
                }
            }

            is_logging_out.set(false);
        });
    };

    let auth_service_for_password = auth_service.clone();
    let toast_store_for_password = toast_store.clone();
    let handle_change_password = move |_| {
        if is_updating_password() {
            return;
        }

        let current = current_password().trim().to_string();
        let next = new_password().trim().to_string();
        let confirm = confirm_password().trim().to_string();

        security_error_message.set(String::new());
        security_success_message.set(String::new());

        if current.is_empty() {
            security_error_message.set("请输入当前密码".to_string());
            return;
        }

        if next.is_empty() {
            security_error_message.set("请输入新密码".to_string());
            return;
        }

        if !(6..=100).contains(&next.len()) {
            security_error_message.set("新密码长度需在 6 到 100 个字符之间".to_string());
            return;
        }

        if next != confirm {
            security_error_message.set("两次输入的新密码不一致".to_string());
            return;
        }

        let auth_service = auth_service_for_password.clone();
        let toast_store = toast_store_for_password.clone();
        spawn(async move {
            is_updating_password.set(true);

            let req = UpdateProfileRequest {
                username: None,
                current_password: current,
                new_password: Some(next),
            };

            match auth_service.change_password(req).await {
                Ok(_) => {
                    current_password.set(String::new());
                    new_password.set(String::new());
                    confirm_password.set(String::new());
                    let message = "密码已更新".to_string();
                    security_success_message.set(message.clone());
                    toast_store.show_success(message);
                }
                Err(err) => {
                    let message = format!("修改密码失败: {}", err);
                    security_error_message.set(message.clone());
                    toast_store.show_error(message);
                }
            }

            is_updating_password.set(false);
        });
    };

    let handle_refresh_system = move |_| {
        if is_loading_system() {
            return;
        }

        system_reload_tick.set(system_reload_tick().wrapping_add(1));
        system_has_loaded.set(false);
    };

    let admin_service_for_deleted_cleanup = admin_service.clone();
    let toast_store_for_deleted_cleanup = toast_store.clone();
    let handle_cleanup_deleted = move |_| {
        if is_cleaning_deleted() || is_cleaning_expired() || is_backing_up() {
            return;
        }

        if !confirm_action("确认永久清理已删除文件吗？该操作会移除文件和对应记录，无法撤销。")
        {
            return;
        }

        let admin_service = admin_service_for_deleted_cleanup.clone();
        let toast_store = toast_store_for_deleted_cleanup.clone();
        spawn(async move {
            is_cleaning_deleted.set(true);
            maintenance_error_message.set(String::new());
            maintenance_success_message.set(String::new());

            match admin_service.cleanup_deleted_files().await {
                Ok(removed) => {
                    let count = removed.len();
                    last_deleted_cleanup_count.set(Some(count));
                    let message = if count == 0 {
                        "当前没有可清理的已删除文件".to_string()
                    } else {
                        format!("已永久清理 {} 个已删除文件", count)
                    };
                    maintenance_success_message.set(message.clone());
                    toast_store.show_success(message);
                }
                Err(err) => {
                    let message = format!("清理已删除文件失败: {}", err);
                    maintenance_error_message.set(message.clone());
                    toast_store.show_error(message);
                }
            }

            is_cleaning_deleted.set(false);
        });
    };

    let admin_service_for_expired_cleanup = admin_service.clone();
    let toast_store_for_expired_cleanup = toast_store.clone();
    let handle_cleanup_expired = move |_| {
        if is_cleaning_deleted() || is_cleaning_expired() || is_backing_up() {
            return;
        }

        if !confirm_action("确认处理所有已过期图片吗？这些图片会被批量移入回收站。")
        {
            return;
        }

        let admin_service = admin_service_for_expired_cleanup.clone();
        let toast_store = toast_store_for_expired_cleanup.clone();
        spawn(async move {
            is_cleaning_expired.set(true);
            maintenance_error_message.set(String::new());
            maintenance_success_message.set(String::new());

            match admin_service.cleanup_expired_images().await {
                Ok(affected) => {
                    last_expired_cleanup_count.set(Some(affected));
                    let message = if affected <= 0 {
                        "当前没有需要处理的过期图片".to_string()
                    } else {
                        format!("已处理 {} 张过期图片", affected)
                    };
                    maintenance_success_message.set(message.clone());
                    toast_store.show_success(message);
                }
                Err(err) => {
                    let message = format!("处理过期图片失败: {}", err);
                    maintenance_error_message.set(message.clone());
                    toast_store.show_error(message);
                }
            }

            is_cleaning_expired.set(false);
        });
    };

    let admin_service_for_backup = admin_service.clone();
    let toast_store_for_backup = toast_store.clone();
    let handle_backup_database = move |_| {
        if is_cleaning_deleted() || is_cleaning_expired() || is_backing_up() {
            return;
        }

        let admin_service = admin_service_for_backup.clone();
        let toast_store = toast_store_for_backup.clone();
        spawn(async move {
            is_backing_up.set(true);
            maintenance_error_message.set(String::new());
            maintenance_success_message.set(String::new());

            match admin_service.backup_database().await {
                Ok(backup) => {
                    let filename = backup.filename.clone();
                    last_backup.set(Some(backup));
                    let message = format!("已生成数据库备份: {}", filename);
                    maintenance_success_message.set(message.clone());
                    toast_store.show_success(message);
                }
                Err(err) => {
                    let message = format!("数据库备份失败: {}", err);
                    maintenance_error_message.set(message.clone());
                    toast_store.show_error(message);
                }
            }

            is_backing_up.set(false);
        });
    };

    let handle_refresh_users = move |_| {
        if is_loading_users() || updating_user_id().is_some() {
            return;
        }
        users_reload_tick.set(users_reload_tick().wrapping_add(1));
    };

    let admin_service_for_user_role = admin_service.clone();
    let toast_store_for_user_role = toast_store.clone();
    let handle_save_user_role = move |user_id: String| {
        if updating_user_id().is_some() {
            return;
        }

        let current_users = users();
        let Some(current_user) = current_users.iter().find(|user| user.id == user_id) else {
            let message = "未找到要更新的用户".to_string();
            users_error_message.set(message.clone());
            toast_store_for_user_role.show_error(message);
            return;
        };

        let next_role = user_role_drafts()
            .get(&user_id)
            .cloned()
            .unwrap_or_else(|| current_user.role.clone());

        if next_role == current_user.role {
            let message = format!("{} 的角色未发生变化", current_user.username);
            users_success_message.set(message.clone());
            toast_store_for_user_role.show_info(message);
            return;
        }

        let username = current_user.username.clone();
        let admin_service = admin_service_for_user_role.clone();
        let toast_store = toast_store_for_user_role.clone();
        spawn(async move {
            updating_user_id.set(Some(user_id.clone()));
            users_error_message.set(String::new());
            users_success_message.set(String::new());

            match admin_service
                .update_user_role(&user_id, next_role.clone())
                .await
            {
                Ok(_) => {
                    let mut current_users = users();
                    if let Some(user) = current_users.iter_mut().find(|user| user.id == user_id) {
                        user.role = next_role.clone();
                    }
                    users.set(current_users);

                    let mut drafts = user_role_drafts();
                    drafts.insert(user_id.clone(), next_role.clone());
                    user_role_drafts.set(drafts);

                    let message = format!("已将 {} 的角色更新为 {}", username, next_role);
                    users_success_message.set(message.clone());
                    toast_store.show_success(message);
                }
                Err(err) => {
                    let message = format!("更新用户角色失败: {}", err);
                    users_error_message.set(message.clone());
                    toast_store.show_error(message);
                }
            }

            updating_user_id.set(None);
        });
    };

    let handle_refresh_audit = move |_| {
        if is_loading_audit() {
            return;
        }
        audit_reload_tick.set(audit_reload_tick().wrapping_add(1));
    };

    let handle_prev_audit_page = move |_| {
        if is_loading_audit() || audit_current_page() <= 1 {
            return;
        }
        audit_current_page.set(audit_current_page() - 1);
    };

    let handle_next_audit_page = move |_| {
        let has_next = (audit_current_page() as i64 * audit_page_size() as i64) < audit_total();
        if is_loading_audit() || !has_next {
            return;
        }
        audit_current_page.set(audit_current_page() + 1);
    };

    let handle_audit_page_size_change = move |value: String| {
        if let Ok(size) = value.parse::<i32>() {
            let normalized_size = size.clamp(1, 100);
            if normalized_size != audit_page_size() {
                audit_page_size.set(normalized_size);
                audit_current_page.set(1);
            }
        }
    };

    let handle_refresh_advanced = move |_| {
        if is_loading_advanced() || saving_setting_key().is_some() {
            return;
        }
        advanced_reload_tick.set(advanced_reload_tick().wrapping_add(1));
    };

    let admin_service_for_raw_setting = admin_service.clone();
    let toast_store_for_raw_setting = toast_store.clone();
    let on_site_name_updated_for_raw_setting = on_site_name_updated.clone();
    let handle_save_setting = move |key: String| {
        if saving_setting_key().is_some() {
            return;
        }

        let current_settings = raw_settings();
        let Some(current_setting) = current_settings.iter().find(|setting| setting.key == key)
        else {
            let message = "未找到要更新的设置项".to_string();
            advanced_error_message.set(message.clone());
            toast_store_for_raw_setting.show_error(message);
            return;
        };

        if !current_setting.editable {
            let message = format!("设置项 {} 受保护，不能通过高级设置直接修改", key);
            advanced_error_message.set(message.clone());
            toast_store_for_raw_setting.show_error(message);
            return;
        }

        let next_value = raw_setting_drafts()
            .get(&key)
            .cloned()
            .unwrap_or_else(|| current_setting.value.clone());

        if next_value == current_setting.value {
            let message = format!("键值 {} 未发生变化", key);
            advanced_success_message.set(message.clone());
            toast_store_for_raw_setting.show_info(message);
            return;
        }

        if current_setting.requires_confirmation
            && !confirm_action(advanced_setting_confirm_message(&key))
        {
            return;
        }

        let admin_service = admin_service_for_raw_setting.clone();
        let toast_store = toast_store_for_raw_setting.clone();
        let on_site_name_updated = on_site_name_updated_for_raw_setting.clone();
        spawn(async move {
            saving_setting_key.set(Some(key.clone()));
            advanced_error_message.set(String::new());
            advanced_success_message.set(String::new());

            match admin_service.update_setting(&key, next_value.clone()).await {
                Ok(_) => {
                    let message = format!("已更新原始设置 {}", key);
                    advanced_success_message.set(message.clone());
                    toast_store.show_success(message);
                    if key == "site_name" {
                        on_site_name_updated.call(next_value.trim().to_string());
                    }
                    reload_tick.set(reload_tick().wrapping_add(1));
                    advanced_reload_tick.set(advanced_reload_tick().wrapping_add(1));
                }
                Err(err) => {
                    let message = format!("更新原始设置失败: {}", err);
                    advanced_error_message.set(message.clone());
                    toast_store.show_error(message);
                }
            }

            saving_setting_key.set(None);
        });
    };

    let settings_sections: &[SettingsSection] = if is_admin {
        &ADMIN_SETTINGS_SECTIONS
    } else {
        &USER_SETTINGS_SECTIONS
    };
    let current_user = auth_store.user();
    let account_username = current_user
        .as_ref()
        .map(|user| user.username.clone())
        .unwrap_or_else(|| "当前用户".to_string());
    let account_role = current_user
        .as_ref()
        .map(|user| user.role.clone())
        .unwrap_or_else(|| "user".to_string());
    let account_created_at = current_user
        .as_ref()
        .map(|user| user.created_at.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "-".to_string());
    let current_section = active_section();
    let is_form_disabled = is_loading() || is_saving();
    let audit_has_next = (audit_current_page() as i64 * audit_page_size() as i64) < audit_total();

    rsx! {
        div { class: "dashboard-page settings-page",
            section { class: "page-hero settings-hero",
                div { class: "settings-hero-main",
                    h1 { "系统设置" }
                }
            }

            div { class: "settings-workspace",
                aside { class: "settings-sidebar",
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
                                }
                            }
                        }
                    }
                }

                div { class: "settings-panel-column",
                    section { class: "settings-card settings-panel-card",
                        h2 { class: "settings-panel-title", "{current_section.title()}" }

                        if !error_message().is_empty() && current_section.uses_global_settings_actions() {
                            div { class: "error-banner", "{error_message()}" }
                        }

                        {
                            match current_section {
                                SettingsSection::Account => rsx! {
                                    AccountSettingsSection {
                                        username: account_username.clone(),
                                        role: account_role.clone(),
                                        created_at: account_created_at.clone(),
                                        is_logging_out: is_logging_out(),
                                        on_logout: handle_logout,
                                    }
                                },
                                SettingsSection::Security => rsx! {
                                    SecuritySettingsSection {
                                        current_password,
                                        new_password,
                                        confirm_password,
                                        error_message: security_error_message(),
                                        success_message: security_success_message(),
                                        is_submitting: is_updating_password(),
                                        on_submit: handle_change_password,
                                    }
                                },
                                SettingsSection::System => rsx! {
                                    SystemStatusSection {
                                        health: system_health(),
                                        stats: system_stats(),
                                        error_message: system_error_message(),
                                        is_loading: is_loading_system(),
                                        on_refresh: handle_refresh_system,
                                    }
                                },
                                SettingsSection::Maintenance => rsx! {
                                    MaintenanceSettingsSection {
                                        error_message: maintenance_error_message(),
                                        success_message: maintenance_success_message(),
                                        last_backup: last_backup(),
                                        last_deleted_cleanup_count: last_deleted_cleanup_count(),
                                        last_expired_cleanup_count: last_expired_cleanup_count(),
                                        is_cleaning_deleted: is_cleaning_deleted(),
                                        is_cleaning_expired: is_cleaning_expired(),
                                        is_backing_up: is_backing_up(),
                                        on_cleanup_deleted: handle_cleanup_deleted,
                                        on_cleanup_expired: handle_cleanup_expired,
                                        on_backup: handle_backup_database,
                                    }
                                },
                                SettingsSection::Users => rsx! {
                                    UsersSettingsSection {
                                        users: users(),
                                        role_drafts: user_role_drafts,
                                        error_message: users_error_message(),
                                        success_message: users_success_message(),
                                        is_loading: is_loading_users(),
                                        updating_user_id: updating_user_id(),
                                        on_refresh: handle_refresh_users,
                                        on_save_role: handle_save_user_role,
                                    }
                                },
                                SettingsSection::Audit => rsx! {
                                    AuditSettingsSection {
                                        logs: audit_logs(),
                                        total: audit_total(),
                                        current_page: audit_current_page(),
                                        page_size: audit_page_size(),
                                        has_next: audit_has_next,
                                        error_message: audit_error_message(),
                                        is_loading: is_loading_audit(),
                                        on_refresh: handle_refresh_audit,
                                        on_prev_page: handle_prev_audit_page,
                                        on_next_page: handle_next_audit_page,
                                        on_page_size_change: handle_audit_page_size_change,
                                    }
                                },
                                SettingsSection::Advanced => rsx! {
                                    AdvancedSettingsSection {
                                        settings: raw_settings(),
                                        setting_drafts: raw_setting_drafts,
                                        error_message: advanced_error_message(),
                                        success_message: advanced_success_message(),
                                        is_loading: is_loading_advanced(),
                                        saving_key: saving_setting_key(),
                                        on_refresh: handle_refresh_advanced,
                                        on_save_setting: handle_save_setting,
                                    }
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
