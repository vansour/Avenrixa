use crate::app_context::{use_install_service, use_toast_store};
use crate::pages::settings_page::{
    SettingsFormState, render_general_fields, render_storage_fields,
};
use crate::types::api::{AdminSettingsConfig, InstallBootstrapRequest, InstallBootstrapResponse};
use base64::Engine;
use dioxus::html::FileData;
use dioxus::prelude::*;

const MIN_ADMIN_PASSWORD_LENGTH: usize = 12;

#[component]
pub fn InstallWizardPage(
    initial_config: AdminSettingsConfig,
    #[props(default)] on_installed: EventHandler<InstallBootstrapResponse>,
) -> Element {
    let install_service = use_install_service();
    let toast_store = use_toast_store();

    let site_name = use_signal({
        let initial = initial_config.site_name.clone();
        move || initial.clone()
    });
    let storage_backend = use_signal({
        let initial = initial_config.storage_backend.clone();
        move || initial.clone()
    });
    let local_storage_path = use_signal({
        let initial = initial_config.local_storage_path.clone();
        move || initial.clone()
    });
    let mail_enabled = use_signal({
        let initial = initial_config.mail_enabled;
        move || initial
    });
    let mail_smtp_host = use_signal({
        let initial = initial_config.mail_smtp_host.clone();
        move || initial.clone()
    });
    let mail_smtp_port = use_signal({
        let initial = initial_config.mail_smtp_port.to_string();
        move || initial.clone()
    });
    let mail_smtp_user = use_signal({
        let initial = initial_config.mail_smtp_user.clone().unwrap_or_default();
        move || initial.clone()
    });
    let mail_smtp_password = use_signal(String::new);
    let mail_smtp_password_set = use_signal({
        let initial = initial_config.mail_smtp_password_set;
        move || initial
    });
    let mail_from_email = use_signal({
        let initial = initial_config.mail_from_email.clone();
        move || initial.clone()
    });
    let mail_from_name = use_signal({
        let initial = initial_config.mail_from_name.clone();
        move || initial.clone()
    });
    let mail_link_base_url = use_signal({
        let initial = initial_config.mail_link_base_url.clone();
        move || initial.clone()
    });
    let s3_endpoint = use_signal({
        let initial = initial_config.s3_endpoint.clone().unwrap_or_default();
        move || initial.clone()
    });
    let s3_region = use_signal({
        let initial = initial_config.s3_region.clone().unwrap_or_default();
        move || initial.clone()
    });
    let s3_bucket = use_signal({
        let initial = initial_config.s3_bucket.clone().unwrap_or_default();
        move || initial.clone()
    });
    let s3_prefix = use_signal({
        let initial = initial_config.s3_prefix.clone().unwrap_or_default();
        move || initial.clone()
    });
    let s3_access_key = use_signal({
        let initial = initial_config.s3_access_key.clone().unwrap_or_default();
        move || initial.clone()
    });
    let s3_secret_key = use_signal(String::new);
    let s3_secret_key_set = use_signal({
        let initial = initial_config.s3_secret_key_set;
        move || initial
    });
    let s3_force_path_style = use_signal({
        let initial = initial_config.s3_force_path_style;
        move || initial
    });

    let form = SettingsFormState {
        site_name,
        storage_backend,
        local_storage_path,
        mail_enabled,
        mail_smtp_host,
        mail_smtp_port,
        mail_smtp_user,
        mail_smtp_password,
        mail_smtp_password_set,
        mail_from_email,
        mail_from_name,
        mail_link_base_url,
        s3_endpoint,
        s3_region,
        s3_bucket,
        s3_prefix,
        s3_access_key,
        s3_secret_key,
        s3_secret_key_set,
        s3_force_path_style,
    };

    let mut admin_email = use_signal(String::new);
    let mut admin_password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut selected_favicon = use_signal(|| None::<FileData>);
    let mut is_installing = use_signal(|| false);
    let mut error_message = use_signal(String::new);
    let mut success_message = use_signal(String::new);

    let handle_pick_favicon = move |event: Event<FormData>| {
        let mut files = event.files().into_iter();
        match files.next() {
            Some(file) => selected_favicon.set(Some(file)),
            None => selected_favicon.set(None),
        }
    };

    let handle_install = move |_| {
        if is_installing() {
            return;
        }

        let email = admin_email().trim().to_string();
        let password = admin_password();
        let confirm = confirm_password();
        if email.is_empty() {
            let message = "请填写管理员邮箱".to_string();
            error_message.set(message.clone());
            toast_store.show_error(message);
            return;
        }
        if password.trim().is_empty() {
            let message = "请填写管理员密码".to_string();
            error_message.set(message.clone());
            toast_store.show_error(message);
            return;
        }
        if password.trim().len() < MIN_ADMIN_PASSWORD_LENGTH {
            let message = format!("管理员密码至少需要 {} 个字符", MIN_ADMIN_PASSWORD_LENGTH);
            error_message.set(message.clone());
            toast_store.show_error(message);
            return;
        }
        if password != confirm {
            let message = "两次输入的管理员密码不一致".to_string();
            error_message.set(message.clone());
            toast_store.show_error(message);
            return;
        }
        if let Err(message) = form.validate() {
            error_message.set(message.clone());
            toast_store.show_error(message);
            return;
        }

        let install_service = install_service.clone();
        let toast_store = toast_store.clone();
        let req_config = form.build_update_request();
        let favicon_file = selected_favicon();

        spawn(async move {
            is_installing.set(true);
            error_message.set(String::new());
            success_message.set(String::new());

            let favicon_data_url = match favicon_file {
                Some(file) => match favicon_file_to_data_url(file).await {
                    Ok(data_url) => Some(data_url),
                    Err(message) => {
                        error_message.set(message.clone());
                        toast_store.show_error(message);
                        is_installing.set(false);
                        return;
                    }
                },
                None => None,
            };

            let request = InstallBootstrapRequest {
                admin_email: email,
                admin_password: password,
                favicon_data_url,
                config: req_config,
            };

            match install_service.bootstrap_installation(request).await {
                Ok(response) => {
                    let message = if response.config.restart_required {
                        "安装完成，管理员已登录；存储配置需要重启服务后生效".to_string()
                    } else {
                        "安装完成，已自动登录管理员账户".to_string()
                    };
                    success_message.set(message.clone());
                    toast_store.show_success(message);
                    on_installed.call(response);
                }
                Err(err) => {
                    let message = format!("安装失败: {}", err);
                    error_message.set(message.clone());
                    toast_store.show_error(message);
                }
            }

            is_installing.set(false);
        });
    };

    let selected_favicon_name = selected_favicon()
        .as_ref()
        .map(|file| file.name())
        .unwrap_or_default();
    let favicon_summary = if selected_favicon_name.is_empty() {
        "未上传"
    } else {
        "已选择"
    };
    let admin_ready = install_admin_ready(admin_email(), admin_password(), confirm_password());
    let site_ready = !(form.site_name)().trim().is_empty();
    let mail_ready = install_mail_ready(form);
    let storage_ready = install_storage_ready(form);
    let ready_count = [admin_ready, site_ready, mail_ready, storage_ready]
        .into_iter()
        .filter(|item| *item)
        .count();
    let storage_summary = if form.is_s3_backend() {
        "S3 / MinIO"
    } else {
        "本地目录"
    };
    let mail_summary = if !(form.mail_enabled)() {
        "未启用"
    } else if mail_ready {
        "已启用"
    } else {
        "待补全"
    };

    rsx! {
        div { class: "dashboard-page settings-page install-page",
            section { class: "page-hero settings-hero settings-hero-rich",
                div { class: "settings-hero-main settings-hero-main-stack",
                    div {
                        p { class: "settings-eyebrow", "First Run Setup" }
                        h1 { "安装向导" }
                        p { class: "settings-hero-copy",
                            "当前实例尚未完成初始化。先创建管理员账户，再写入站点、邮件与存储配置，安装完成后会自动登录管理员。"
                        }
                    }
                    div { class: "settings-pill-row",
                        span { class: "stat-pill stat-pill-active", "初始化进度 {ready_count}/4" }
                        span { class: "stat-pill", "登录方式：邮箱" }
                        span { class: "stat-pill", "存储：{storage_summary}" }
                        span { class: "stat-pill", "邮件：{mail_summary}" }
                    }
                }
            }

            div { class: "settings-workspace settings-workspace-install",
                aside { class: "settings-sidebar",
                    div { class: "settings-sidebar-card settings-sidebar-intro",
                        p { class: "settings-eyebrow", "部署摘要" }
                        h2 { "安装前检查" }
                        p { class: "settings-sidebar-copy",
                            "这个阶段只负责创建首个管理员并写入站点配置；数据库连接已经在上一阶段完成。"
                        }
                        div { class: "settings-sidebar-meta",
                            span { class: "stat-pill", "管理员：邮箱登录" }
                            span { class: "stat-pill", "图标：{favicon_summary}" }
                        }
                    }

                    div { class: "settings-sidebar-card settings-checklist-card",
                        h3 { "就绪检查" }
                        p { class: "settings-section-copy",
                            "四项全部完成后即可提交安装。"
                        }
                        div { class: "settings-checklist" ,
                            {render_install_check_item("管理员账户", "邮箱已填写且密码满足强度要求", admin_ready)}
                            {render_install_check_item("站点识别", "站点名称已填写", site_ready)}
                            {render_install_check_item("邮件配置", "关闭邮件，或 SMTP / 跳转地址已补全", mail_ready)}
                            {render_install_check_item("存储配置", "当前存储后端所需字段已补全", storage_ready)}
                        }
                    }

                    div { class: "settings-sidebar-card",
                        h3 { "安装后立即生效" }
                        div { class: "settings-metric-grid",
                            article { class: "settings-metric-card",
                                p { class: "settings-summary-label", "管理员登录方式" }
                                h3 { "邮箱 + 密码" }
                            }
                            article { class: "settings-metric-card",
                                p { class: "settings-summary-label", "站点名称" }
                                h3 { "{summary_or_pending((form.site_name)())}" }
                            }
                            article { class: "settings-metric-card",
                                p { class: "settings-summary-label", "存储后端" }
                                h3 { "{storage_summary}" }
                            }
                        }
                    }
                }

                div { class: "settings-panel-column",
                    section { class: "settings-card",
                        div { class: "settings-panel-head",
                            div {
                                h2 { class: "settings-panel-title", "管理员账户" }
                                p { class: "settings-panel-copy",
                                    "这个账户会成为首个管理员。当前系统只使用邮箱作为登录标识，不再单独维护用户名。"
                                }
                            }
                        }
                        div { class: "settings-banner settings-banner-neutral",
                            "管理员密码至少需要 12 位，建议包含大小写字母、数字与符号。"
                        }
                        div { class: "settings-grid",
                            label { class: "settings-field settings-field-full",
                                span { "管理员邮箱" }
                                input {
                                    r#type: "email",
                                    value: "{admin_email()}",
                                    oninput: move |event| admin_email.set(event.value()),
                                    disabled: is_installing(),
                                }
                            }

                            label { class: "settings-field",
                                span { "管理员密码" }
                                input {
                                    r#type: "password",
                                    value: "{admin_password()}",
                                    oninput: move |event| admin_password.set(event.value()),
                                    disabled: is_installing(),
                                }
                            }

                            label { class: "settings-field",
                                span { "确认密码" }
                                input {
                                    r#type: "password",
                                    value: "{confirm_password()}",
                                    oninput: move |event| confirm_password.set(event.value()),
                                    disabled: is_installing(),
                                }
                            }
                        }
                    }

                    section { class: "settings-card",
                        div { class: "settings-panel-head",
                            div {
                                h2 { class: "settings-panel-title", "站点图标" }
                                p { class: "settings-panel-copy",
                                    "图标会用于浏览器标签和安装完成后的站点识别。支持 ico / png / svg / webp / jpeg。"
                                }
                            }
                        }
                        div { class: "settings-grid",
                            label { class: "settings-field settings-field-full",
                                span { "网站图标（可选）" }
                                input {
                                    r#type: "file",
                                    accept: ".ico,image/png,image/svg+xml,image/webp,image/jpeg,image/x-icon,image/vnd.microsoft.icon",
                                    onchange: handle_pick_favicon,
                                    disabled: is_installing(),
                                }
                            }
                            if !selected_favicon_name.is_empty() {
                                p { class: "install-file-meta settings-field-full",
                                    "已选择图标：{selected_favicon_name}"
                                }
                            } else {
                                p { class: "install-file-meta settings-field-full",
                                    "暂未上传图标，后续可以在管理员设置里继续调整。"
                                }
                            }
                        }
                    }

                    section { class: "settings-card",
                        div { class: "settings-panel-head",
                            div {
                                h2 { class: "settings-panel-title", "站点与邮件" }
                                p { class: "settings-panel-copy",
                                    "定义站点名称、发件人信息和邮箱验证/密码找回跳转地址。"
                                }
                            }
                        }
                        {render_general_fields(form, is_installing())}
                    }

                    section { class: "settings-card",
                        div { class: "settings-panel-head",
                            div {
                                h2 { class: "settings-panel-title", "存储配置" }
                                p { class: "settings-panel-copy",
                                    "决定图片写入本地目录还是对象存储。保存后如果提示需要重启，再进行服务重启。"
                                }
                            }
                        }
                        {render_storage_fields(form, is_installing())}
                    }

                    if !success_message().is_empty() {
                        div { class: "settings-banner settings-banner-success", "{success_message()}" }
                    }
                    if !error_message().is_empty() {
                        div { class: "error-banner", "{error_message()}" }
                    }

                    div { class: "settings-actions settings-actions-split",
                        p { class: "settings-section-copy",
                            "提交后会写入安装状态并自动登录管理员。"
                        }
                        button {
                            class: "btn btn-primary",
                            r#type: "button",
                            onclick: handle_install,
                            disabled: is_installing(),
                            if is_installing() {
                                "正在安装..."
                            } else {
                                "完成安装"
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn favicon_file_to_data_url(file: FileData) -> Result<String, String> {
    let mime = infer_favicon_mime(&file);
    let bytes = file
        .read_bytes()
        .await
        .map_err(|err| format!("读取网站图标失败: {}", err))?;
    if bytes.is_empty() {
        return Err("网站图标内容为空".to_string());
    }

    Ok(format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

fn infer_favicon_mime(file: &FileData) -> &'static str {
    if let Some(content_type) = file.content_type() {
        match content_type.trim().to_ascii_lowercase().as_str() {
            "image/x-icon" | "image/vnd.microsoft.icon" | "image/ico" => {
                return "image/x-icon";
            }
            "image/png" => return "image/png",
            "image/svg+xml" => return "image/svg+xml",
            "image/webp" => return "image/webp",
            "image/jpeg" | "image/jpg" => return "image/jpeg",
            _ => {}
        }
    }

    let filename = file.name().to_ascii_lowercase();
    if filename.ends_with(".ico") {
        "image/x-icon"
    } else if filename.ends_with(".svg") {
        "image/svg+xml"
    } else if filename.ends_with(".webp") {
        "image/webp"
    } else if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "image/png"
    }
}

fn install_admin_ready(email: String, password: String, confirm: String) -> bool {
    let email = email.trim();
    let password = password.trim();
    !email.is_empty()
        && email.contains('@')
        && password.len() >= MIN_ADMIN_PASSWORD_LENGTH
        && password == confirm
}

fn install_mail_ready(form: SettingsFormState) -> bool {
    if !(form.mail_enabled)() {
        return true;
    }

    let smtp_host = (form.mail_smtp_host)().trim().to_string();
    let smtp_port = (form.mail_smtp_port)().trim().to_string();
    let smtp_user = (form.mail_smtp_user)().trim().to_string();
    let smtp_password = (form.mail_smtp_password)().trim().to_string();
    let from_email = (form.mail_from_email)().trim().to_string();
    let link_base_url = (form.mail_link_base_url)().trim().to_string();
    let password_ready =
        !smtp_password.is_empty() || ((form.mail_smtp_password_set)() && !smtp_user.is_empty());

    !smtp_host.is_empty()
        && !from_email.is_empty()
        && !link_base_url.is_empty()
        && smtp_port
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)
            .is_some()
        && (smtp_user.is_empty() == password_ready)
}

fn install_storage_ready(form: SettingsFormState) -> bool {
    if !form.is_s3_backend() {
        return !(form.local_storage_path)().trim().is_empty();
    }

    !(form.local_storage_path)().trim().is_empty()
        && !(form.s3_endpoint)().trim().is_empty()
        && !(form.s3_region)().trim().is_empty()
        && !(form.s3_bucket)().trim().is_empty()
        && !(form.s3_access_key)().trim().is_empty()
        && ((form.s3_secret_key_set)() || !(form.s3_secret_key)().trim().is_empty())
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

fn summary_or_pending(value: String) -> String {
    let value = value.trim().to_string();
    if value.is_empty() {
        "待填写".to_string()
    } else {
        value
    }
}
