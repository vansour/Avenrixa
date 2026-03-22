use crate::app_context::{use_install_service, use_toast_store};
use crate::types::api::{
    BootstrapDatabaseKind, BootstrapStatusResponse, UpdateBootstrapDatabaseConfigRequest,
};
use dioxus::prelude::*;

#[component]
pub fn BootstrapDatabasePage(
    status: BootstrapStatusResponse,
    #[props(default)] on_status_updated: EventHandler<BootstrapStatusResponse>,
) -> Element {
    let install_service = use_install_service();
    let toast_store = use_toast_store();
    let initial_database_kind = match status.database_kind {
        BootstrapDatabaseKind::Unknown => BootstrapDatabaseKind::Postgres,
        kind => kind,
    };

    let mut database_kind = use_signal(|| initial_database_kind);
    let mut database_url = use_signal(String::new);
    let mut is_saving = use_signal(|| false);
    let mut error_message = use_signal(String::new);
    let mut success_message = use_signal(String::new);
    let database_target_label = "数据库连接 URL";
    let database_target_placeholder = "postgresql://user:pass@host:5432/dbname".to_string();
    let page_title = "当前实例启动时未检测到预设的 PostgreSQL 数据库连接。这个页面只是兜底引导：写入连接信息并重启服务后，再继续管理员安装。";
    let save_button_label = "保存数据库配置";

    let handle_save = move |_| {
        if is_saving() {
            return;
        }

        let current_database_kind = database_kind();
        let url = database_url().trim().to_string();
        if url.is_empty() {
            let message = "请填写 PostgreSQL 数据库连接 URL".to_string();
            error_message.set(message.clone());
            toast_store.show_error(message);
            return;
        }

        let install_service = install_service.clone();
        let toast_store = toast_store.clone();

        spawn(async move {
            is_saving.set(true);
            error_message.set(String::new());
            success_message.set(String::new());

            match install_service
                .update_bootstrap_database_config(UpdateBootstrapDatabaseConfigRequest {
                    database_kind: current_database_kind,
                    database_url: url,
                })
                .await
            {
                Ok(response) => {
                    let message = "数据库配置已保存，请重启服务后继续安装".to_string();
                    success_message.set(message.clone());
                    toast_store.show_success(message);
                    on_status_updated.call(BootstrapStatusResponse {
                        mode: "bootstrap".to_string(),
                        database_kind: response.database_kind,
                        database_configured: response.database_configured,
                        database_url_masked: Some(response.database_url_masked),
                        cache_configured: false,
                        cache_url_masked: None,
                        restart_required: response.restart_required,
                        runtime_error: None,
                    });
                }
                Err(err) => {
                    let message = format!("保存数据库配置失败: {}", err);
                    error_message.set(message.clone());
                    toast_store.show_error(message);
                }
            }

            is_saving.set(false);
        });
    };

    rsx! {
        div { class: "dashboard-page settings-page install-page",
            section { class: "page-hero settings-hero",
                div { class: "settings-hero-main",
                    div {
                        h1 { "数据库引导" }
                        p { "{page_title}" }
                    }
                }
            }

            section { class: "settings-card",
                h2 { class: "settings-panel-title", "数据库连接" }
                div { class: "settings-grid",
                    label { class: "settings-field",
                        span { "数据库类型" }
                        select {
                            value: "{database_kind().as_str()}",
                            onchange: move |event| {
                                let next = BootstrapDatabaseKind::parse(&event.value());
                                database_kind.set(match next {
                                    BootstrapDatabaseKind::Unknown => BootstrapDatabaseKind::Postgres,
                                    kind => kind,
                                });
                            },
                            disabled: is_saving(),
                            option { value: "postgresql", "PostgreSQL" }
                        }
                    }

                    label { class: "settings-field settings-field-full",
                        span { "{database_target_label}" }
                        input {
                            r#type: "text",
                            placeholder: "{database_target_placeholder}",
                            value: "{database_url()}",
                            oninput: move |event| database_url.set(event.value()),
                            disabled: is_saving(),
                        }
                    }
                }

                if let Some(masked) = status.database_url_masked.as_ref() {
                    p { class: "install-file-meta",
                        "当前已保存的数据库配置摘要：{masked}"
                    }
                }
                p { class: "install-file-meta",
                    "如果你使用 Docker Compose 或其他可控部署入口，优先通过环境变量 `DATABASE_KIND` / `DATABASE_URL` 预设数据库连接；只有未预设时才需要使用这个页面。"
                }
                p { class: "install-file-meta",
                    "PostgreSQL 模式会在保存时校验连接，并在服务重启后继续安装流程。"
                }
                if let Some(runtime_error) = status.runtime_error.as_ref() {
                    p { class: "upload-message upload-message-error",
                        "最近一次启动连接数据库失败：{runtime_error}"
                    }
                }
                if status.restart_required {
                    p { class: "upload-message upload-message-success",
                        "数据库配置文件已存在。修改后仍需重启服务才能继续。"
                    }
                }
            }

            section { class: "settings-card",
                h2 { class: "settings-panel-title", "下一步" }
                div { class: "settings-stack",
                    p { class: "login-subtitle",
                        "1. 优先在部署环境里设置 `DATABASE_KIND=postgresql` 和 `DATABASE_URL`；只有未预设时才在这里填写并保存 PostgreSQL 连接。"
                    }
                    p { class: "login-subtitle",
                        "2. 重启应用服务。"
                    }
                    p { class: "login-subtitle",
                        "3. 重启后页面会自动进入管理员安装向导。"
                    }
                }
            }

            if !success_message().is_empty() {
                p { class: "upload-message upload-message-success", "{success_message()}" }
            }
            if !error_message().is_empty() {
                p { class: "upload-message upload-message-error", "{error_message()}" }
            }

            div { class: "settings-actions",
                button {
                    class: "btn btn-primary",
                    r#type: "button",
                    onclick: handle_save,
                    disabled: is_saving(),
                    if is_saving() {
                        "正在校验并保存..."
                    } else {
                        "{save_button_label}"
                    }
                }
            }
        }
    }
}
