use crate::app_context::{use_settings_service, use_toast_store};
use crate::types::api::{AdminSettingsConfig, UpdateAdminSettingsConfigRequest};
use dioxus::prelude::*;

fn apply_loaded_config(
    config: AdminSettingsConfig,
    site_name: &mut Signal<String>,
    storage_backend: &mut Signal<String>,
    local_storage_path: &mut Signal<String>,
    s3_endpoint: &mut Signal<String>,
    s3_region: &mut Signal<String>,
    s3_bucket: &mut Signal<String>,
    s3_prefix: &mut Signal<String>,
    s3_access_key: &mut Signal<String>,
    s3_secret_key: &mut Signal<String>,
    s3_secret_key_set: &mut Signal<bool>,
    s3_force_path_style: &mut Signal<bool>,
) {
    site_name.set(config.site_name);
    storage_backend.set(config.storage_backend);
    local_storage_path.set(config.local_storage_path);
    s3_endpoint.set(config.s3_endpoint.unwrap_or_default());
    s3_region.set(config.s3_region.unwrap_or_default());
    s3_bucket.set(config.s3_bucket.unwrap_or_default());
    s3_prefix.set(config.s3_prefix.unwrap_or_default());
    s3_access_key.set(config.s3_access_key.unwrap_or_default());
    s3_secret_key.set(String::new());
    s3_secret_key_set.set(config.s3_secret_key_set);
    s3_force_path_style.set(config.s3_force_path_style);
}

#[component]
pub fn SettingsPage(#[props(default)] on_site_name_updated: EventHandler<String>) -> Element {
    let settings_service = use_settings_service();
    let toast_store = use_toast_store();

    let mut is_loading = use_signal(|| true);
    let mut is_saving = use_signal(|| false);
    let mut error_message = use_signal(String::new);

    let mut site_name = use_signal(String::new);
    let mut storage_backend = use_signal(|| "local".to_string());
    let mut local_storage_path = use_signal(String::new);
    let mut s3_endpoint = use_signal(String::new);
    let mut s3_region = use_signal(String::new);
    let mut s3_bucket = use_signal(String::new);
    let mut s3_prefix = use_signal(String::new);
    let mut s3_access_key = use_signal(String::new);
    let mut s3_secret_key = use_signal(String::new);
    let mut s3_secret_key_set = use_signal(|| false);
    let mut s3_force_path_style = use_signal(|| true);
    let mut reload_tick = use_signal(|| 0_u64);

    let _load_settings = use_resource({
        let settings_service = settings_service.clone();
        let toast_store = toast_store.clone();
        move || {
            let _ = reload_tick();
            let settings_service = settings_service.clone();
            let toast_store = toast_store.clone();
            async move {
                is_loading.set(true);
                error_message.set(String::new());

                match settings_service.get_admin_settings_config().await {
                    Ok(config) => {
                        apply_loaded_config(
                            config,
                            &mut site_name,
                            &mut storage_backend,
                            &mut local_storage_path,
                            &mut s3_endpoint,
                            &mut s3_region,
                            &mut s3_bucket,
                            &mut s3_prefix,
                            &mut s3_access_key,
                            &mut s3_secret_key,
                            &mut s3_secret_key_set,
                            &mut s3_force_path_style,
                        );
                    }
                    Err(err) => {
                        let message = format!("加载设置失败: {}", err);
                        error_message.set(message.clone());
                        toast_store.show_error(message);
                    }
                }
                is_loading.set(false);
            }
        }
    });

    let handle_save = move |_| {
        if is_saving() {
            return;
        }

        let site_name_val = site_name().trim().to_string();
        let local_path_val = local_storage_path().trim().to_string();
        let backend_val = storage_backend();
        if site_name_val.is_empty() || local_path_val.is_empty() {
            let message = "网站名称和本地存储路径不能为空".to_string();
            error_message.set(message.clone());
            toast_store.show_error(message);
            return;
        }

        if backend_val == "s3"
            && (s3_endpoint().trim().is_empty()
                || s3_region().trim().is_empty()
                || s3_bucket().trim().is_empty()
                || s3_access_key().trim().is_empty()
                || (!s3_secret_key_set() && s3_secret_key().trim().is_empty()))
        {
            let message =
                "S3 模式下请填写 endpoint/region/bucket/access_key/secret_key".to_string();
            error_message.set(message.clone());
            toast_store.show_error(message);
            return;
        }

        let req = UpdateAdminSettingsConfigRequest {
            site_name: site_name_val,
            storage_backend: backend_val.clone(),
            local_storage_path: local_path_val,
            s3_endpoint: Some(s3_endpoint().trim().to_string()).filter(|v| !v.is_empty()),
            s3_region: Some(s3_region().trim().to_string()).filter(|v| !v.is_empty()),
            s3_bucket: Some(s3_bucket().trim().to_string()).filter(|v| !v.is_empty()),
            s3_prefix: Some(s3_prefix().trim().to_string()).filter(|v| !v.is_empty()),
            s3_access_key: Some(s3_access_key().trim().to_string()).filter(|v| !v.is_empty()),
            s3_secret_key: Some(s3_secret_key().trim().to_string()).filter(|v| !v.is_empty()),
            s3_force_path_style: Some(s3_force_path_style()),
        };

        let settings_service = settings_service.clone();
        let toast_store = toast_store.clone();
        let on_site_name_updated = on_site_name_updated.clone();
        spawn(async move {
            is_saving.set(true);
            error_message.set(String::new());

            match settings_service.update_admin_settings_config(req).await {
                Ok(config) => {
                    apply_loaded_config(
                        config.clone(),
                        &mut site_name,
                        &mut storage_backend,
                        &mut local_storage_path,
                        &mut s3_endpoint,
                        &mut s3_region,
                        &mut s3_bucket,
                        &mut s3_prefix,
                        &mut s3_access_key,
                        &mut s3_secret_key,
                        &mut s3_secret_key_set,
                        &mut s3_force_path_style,
                    );
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

    rsx! {
        div { class: "dashboard-page settings-page",
            section { class: "page-hero",
                h1 { "管理员设置" }
                p { "配置网站名称、存储后端与对象存储连接参数。" }
            }

            section { class: "settings-card",
                if !error_message().is_empty() {
                    div { class: "error-banner", "{error_message()}" }
                }

                div { class: "settings-grid",
                    label { class: "settings-field",
                        span { "网站名称" }
                        input {
                            r#type: "text",
                            value: "{site_name}",
                            oninput: move |e| site_name.set(e.value()),
                            disabled: is_loading() || is_saving(),
                        }
                    }

                    label { class: "settings-field",
                        span { "存储后端" }
                        select {
                            value: "{storage_backend()}",
                            onchange: move |e| storage_backend.set(e.value()),
                            disabled: is_loading() || is_saving(),
                            option { value: "local", "local" }
                            option { value: "s3", "s3" }
                        }
                    }

                    label { class: "settings-field settings-field-full",
                        span { "本地存储路径" }
                        input {
                            r#type: "text",
                            value: "{local_storage_path}",
                            oninput: move |e| local_storage_path.set(e.value()),
                            disabled: is_loading() || is_saving(),
                        }
                    }

                    if storage_backend() == "s3" {
                        label { class: "settings-field",
                            span { "S3 Endpoint" }
                            input {
                                r#type: "text",
                                value: "{s3_endpoint}",
                                oninput: move |e| s3_endpoint.set(e.value()),
                                disabled: is_loading() || is_saving(),
                            }
                        }
                        label { class: "settings-field",
                            span { "S3 Region" }
                            input {
                                r#type: "text",
                                value: "{s3_region}",
                                oninput: move |e| s3_region.set(e.value()),
                                disabled: is_loading() || is_saving(),
                            }
                        }
                        label { class: "settings-field",
                            span { "S3 Bucket" }
                            input {
                                r#type: "text",
                                value: "{s3_bucket}",
                                oninput: move |e| s3_bucket.set(e.value()),
                                disabled: is_loading() || is_saving(),
                            }
                        }
                        label { class: "settings-field",
                            span { "S3 Prefix (可选)" }
                            input {
                                r#type: "text",
                                value: "{s3_prefix}",
                                oninput: move |e| s3_prefix.set(e.value()),
                                disabled: is_loading() || is_saving(),
                            }
                        }
                        label { class: "settings-field",
                            span { "S3 Access Key" }
                            input {
                                r#type: "text",
                                value: "{s3_access_key}",
                                oninput: move |e| s3_access_key.set(e.value()),
                                disabled: is_loading() || is_saving(),
                            }
                        }
                        label { class: "settings-field",
                            span { if s3_secret_key_set() { "S3 Secret Key（留空表示不修改）" } else { "S3 Secret Key" } }
                            input {
                                r#type: "password",
                                value: "{s3_secret_key}",
                                oninput: move |e| s3_secret_key.set(e.value()),
                                disabled: is_loading() || is_saving(),
                            }
                        }
                        label { class: "settings-check",
                            input {
                                r#type: "checkbox",
                                checked: s3_force_path_style(),
                                onchange: move |e| s3_force_path_style.set(e.checked()),
                                disabled: is_loading() || is_saving(),
                            }
                            span { "S3 Force Path Style（MinIO 通常需要开启）" }
                        }
                    }
                }

                div { class: "settings-actions",
                    button {
                        class: "btn",
                        onclick: handle_refresh,
                        disabled: is_loading() || is_saving(),
                        if is_loading() {
                            "加载中..."
                        } else {
                            "刷新"
                        }
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: handle_save,
                        disabled: is_loading() || is_saving(),
                        if is_saving() {
                            "保存中..."
                        } else {
                            "保存设置"
                        }
                    }
                }
            }
        }
    }
}
