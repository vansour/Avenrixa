mod browser;

use crate::types::api::StorageBackendKind;
use dioxus::prelude::*;

use self::browser::LocalStoragePathPicker;
use super::super::state::SettingsFormState;
use super::forms::render_s3_fields;
use super::shared::{render_metric_card, summary_value};

pub fn render_storage_fields(form: SettingsFormState, disabled: bool) -> Element {
    let mut storage_backend = form.storage_backend;
    let local_storage_path = form.local_storage_path;
    let show_s3_fields = form.is_s3_backend();
    let backend_label = if show_s3_fields {
        StorageBackendKind::S3.label().to_string()
    } else {
        StorageBackendKind::Local.label().to_string()
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
                "存储后端切换属于运行时关键配置。保存后会立即应用，建议立刻检查目录权限或对象存储参数。"
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
                            value: "{storage_backend().as_str()}",
                            onchange: move |event| {
                                storage_backend.set(StorageBackendKind::parse(&event.value()))
                            },
                            disabled,
                            option { value: StorageBackendKind::Local.as_str(), "本地存储" }
                            option { value: StorageBackendKind::S3.as_str(), "对象存储（S3）" }
                        }
                    }
                    LocalStoragePathPicker { form, disabled }
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
                            value: "{storage_backend().as_str()}",
                            onchange: move |event| {
                                storage_backend.set(StorageBackendKind::parse(&event.value()))
                            },
                            disabled,
                            option { value: StorageBackendKind::Local.as_str(), "本地存储" }
                            option { value: StorageBackendKind::S3.as_str(), "对象存储（S3）" }
                        }
                    }
                    LocalStoragePathPicker { form, disabled }
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
