use dioxus::prelude::*;

use super::super::super::state::{S3ProviderPreset, SettingsFormState};

pub fn render_s3_fields(form: SettingsFormState, disabled: bool) -> Element {
    render_s3_fields_inner(form, disabled)
}

pub fn render_s3_fields_compact(form: SettingsFormState, disabled: bool) -> Element {
    render_s3_fields_inner(form, disabled)
}

fn render_s3_fields_inner(form: SettingsFormState, disabled: bool) -> Element {
    let mut s3_endpoint = form.s3_endpoint;
    let mut s3_region = form.s3_region;
    let mut s3_bucket = form.s3_bucket;
    let mut s3_prefix = form.s3_prefix;
    let mut s3_access_key = form.s3_access_key;
    let mut s3_secret_key = form.s3_secret_key;
    let s3_secret_key_set = form.s3_secret_key_set;
    let mut s3_force_path_style = form.s3_force_path_style;
    let selected_preset = (form.s3_provider_preset)();

    rsx! {
        label { class: "settings-field",
            span { "提供商预设" }
            select {
                value: "{selected_preset.as_str()}",
                onchange: move |event| {
                    let preset = S3ProviderPreset::parse(&event.value());
                    form.switch_s3_provider_preset(preset);
                },
                disabled,
                option { value: "aws-s3", "AWS S3" }
                option { value: "cloudflare-r2", "Cloudflare R2" }
                option { value: "minio", "MinIO" }
                option { value: "other", "其他兼容服务" }
            }
            small { class: "settings-field-hint", "{selected_preset.endpoint_hint()}" }
        }
        label { class: "settings-field",
            span { "服务地址（必填）" }
            input {
                r#type: "text",
                placeholder: "{selected_preset.endpoint_placeholder()}",
                value: "{s3_endpoint()}",
                oninput: move |event| s3_endpoint.set(event.value()),
                disabled,
            }
            small { class: "settings-field-hint", "按当前提供商填写 endpoint。" }
        }
        label { class: "settings-field",
            span { "存储区域（必填）" }
            input {
                r#type: "text",
                placeholder: "{selected_preset.region_placeholder()}",
                value: "{s3_region()}",
                oninput: move |event| s3_region.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-field",
            span { "存储桶（必填）" }
            input {
                r#type: "text",
                placeholder: "vansour-image",
                value: "{s3_bucket()}",
                oninput: move |event| s3_bucket.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-field",
            span { "目录前缀（可选）" }
            input {
                r#type: "text",
                placeholder: "images/prod",
                value: "{s3_prefix()}",
                oninput: move |event| s3_prefix.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-field",
            span { "访问密钥 ID（必填）" }
            input {
                r#type: "text",
                placeholder: "AKIA...",
                value: "{s3_access_key()}",
                oninput: move |event| s3_access_key.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-field",
            span {
                if s3_secret_key_set() {
                    "机密访问密钥（留空不修改）"
                } else {
                    "机密访问密钥"
                }
            }
            input {
                r#type: "password",
                placeholder: if s3_secret_key_set() {
                    "留空表示继续使用现有机密密钥"
                } else {
                    "输入机密访问密钥"
                },
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
            span { "{selected_preset.path_style_hint()}" }
        }
    }
}
