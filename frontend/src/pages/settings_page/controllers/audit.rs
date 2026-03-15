use crate::app_context::{use_admin_service, use_auth_store, use_toast_store};
use crate::types::api::{AuditLog, PaginationParams};
use dioxus::prelude::*;

use super::super::view::AuditSettingsSection;
use super::set_settings_load_error;

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
        let has_next = current_page() as i64 * (page_size() as i64) < total();
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

    let has_next = current_page() as i64 * (page_size() as i64) < total();

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
