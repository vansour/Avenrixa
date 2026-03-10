use crate::app_context::{use_install_service, use_toast_store};
use crate::types::api::{BootstrapStatusResponse, UpdateBootstrapDatabaseConfigRequest};
use dioxus::prelude::*;

#[component]
pub fn BootstrapDatabasePage(
    status: BootstrapStatusResponse,
    #[props(default)] on_status_updated: EventHandler<BootstrapStatusResponse>,
) -> Element {
    let install_service = use_install_service();
    let toast_store = use_toast_store();
    let initial_database_kind = if status.database_kind.eq_ignore_ascii_case("sqlite") {
        "sqlite"
    } else {
        "postgresql"
    };

    let mut database_kind = use_signal(|| initial_database_kind.to_string());
    let mut database_url = use_signal(String::new);
    let mut database_max_connections = use_signal(|| {
        status
            .database_max_connections
            .unwrap_or(10)
            .max(1)
            .to_string()
    });
    let mut is_saving = use_signal(|| false);
    let mut error_message = use_signal(String::new);
    let mut success_message = use_signal(String::new);
    let selected_database_kind = database_kind();
    let is_sqlite = selected_database_kind == "sqlite";
    let database_target_label = if is_sqlite {
        "数据库文件"
    } else {
        "数据库连接 URL"
    };
    let database_target_placeholder = if is_sqlite {
        "/data/sqlite/app.db 或 sqlite:///data/sqlite/app.db"
    } else {
        "postgresql://user:pass@host:5432/dbname"
    };
    let page_title = if is_sqlite {
        "当前实例尚未连接数据库。先写入 SQLite 数据库文件位置，重启服务后会自动执行迁移并进入安装向导。"
    } else {
        "当前实例尚未连接数据库。先写入 PostgreSQL 连接信息，重启服务后再继续管理员安装。"
    };
    let save_button_label = if is_sqlite {
        "保存 SQLite 配置"
    } else {
        "保存数据库配置"
    };

    let handle_save = move |_| {
        if is_saving() {
            return;
        }

        let current_database_kind = database_kind();
        let url = database_url().trim().to_string();
        if url.is_empty() {
            let message = if current_database_kind == "sqlite" {
                "请填写 SQLite 数据库文件路径或 sqlite:// 连接".to_string()
            } else {
                "请填写 PostgreSQL 数据库连接 URL".to_string()
            };
            error_message.set(message.clone());
            toast_store.show_error(message);
            return;
        }

        let max_connections = if current_database_kind == "sqlite" {
            None
        } else {
            database_max_connections()
                .trim()
                .parse::<u32>()
                .ok()
                .filter(|value| *value > 0)
        };

        let install_service = install_service.clone();
        let toast_store = toast_store.clone();

        spawn(async move {
            is_saving.set(true);
            error_message.set(String::new());
            success_message.set(String::new());

            match install_service
                .update_bootstrap_database_config(UpdateBootstrapDatabaseConfigRequest {
                    database_kind: current_database_kind.clone(),
                    database_url: url,
                    database_max_connections: max_connections,
                })
                .await
            {
                Ok(response) => {
                    let message = if current_database_kind == "sqlite" {
                        "SQLite 配置已保存，请重启服务后继续安装".to_string()
                    } else {
                        "数据库配置已保存，请重启服务后继续安装".to_string()
                    };
                    success_message.set(message.clone());
                    toast_store.show_success(message);
                    on_status_updated.call(BootstrapStatusResponse {
                        mode: "bootstrap".to_string(),
                        database_kind: response.database_kind,
                        database_configured: response.database_configured,
                        database_url_masked: Some(response.database_url_masked),
                        database_max_connections: Some(response.database_max_connections),
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
                            value: "{selected_database_kind}",
                            onchange: move |event| database_kind.set(event.value()),
                            disabled: is_saving(),
                            option { value: "postgresql", "PostgreSQL" }
                            option { value: "sqlite", "SQLite" }
                        }
                    }

                    label { class: "settings-field",
                        span { "最大连接数（可选）" }
                        input {
                            r#type: "number",
                            min: "1",
                            value: "{database_max_connections()}",
                            oninput: move |event| database_max_connections.set(event.value()),
                            disabled: is_saving() || is_sqlite,
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
                        "当前已保存的数据库配置：{masked}"
                    }
                }
                if is_sqlite {
                    p { class: "install-file-meta",
                        "SQLite 模式会在保存时校验数据库文件可创建、可打开，并在服务启动时自动执行 SQLite migrations。"
                    }
                } else {
                    p { class: "install-file-meta",
                        "PostgreSQL 模式会在保存时校验连接，并在服务重启后继续安装流程。"
                    }
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
                    if is_sqlite {
                        p { class: "login-subtitle",
                            "1. 选择 SQLite 并填写数据库文件位置。"
                        }
                        p { class: "login-subtitle",
                            "2. 保存配置，确认数据库文件可以创建并通过基础查询校验。"
                        }
                        p { class: "login-subtitle",
                            "3. 重启应用服务，系统会自动执行 SQLite migrations 并进入安装向导。"
                        }
                    } else {
                        p { class: "login-subtitle",
                            "1. 填写并保存 PostgreSQL 连接。"
                        }
                        p { class: "login-subtitle",
                            "2. 重启应用服务。"
                        }
                        p { class: "login-subtitle",
                            "3. 重启后页面会自动进入管理员安装向导。"
                        }
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
