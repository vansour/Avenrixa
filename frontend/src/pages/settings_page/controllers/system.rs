use crate::app_context::{use_admin_service, use_auth_store, use_toast_store};
use crate::types::api::{HealthStatus, SystemStats};
use dioxus::prelude::*;

use super::super::view::SystemStatusSection;
use super::super::{handle_settings_auth_error, settings_auth_expired_message};

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
