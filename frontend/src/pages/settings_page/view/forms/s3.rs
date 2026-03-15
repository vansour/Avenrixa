use dioxus::prelude::*;

use super::super::super::state::SettingsFormState;

pub fn render_s3_fields(form: SettingsFormState, disabled: bool) -> Element {
    let mut s3_endpoint = form.s3_endpoint;
    let mut s3_region = form.s3_region;
    let mut s3_bucket = form.s3_bucket;
    let mut s3_prefix = form.s3_prefix;
    let mut s3_access_key = form.s3_access_key;
    let mut s3_secret_key = form.s3_secret_key;
    let s3_secret_key_set = form.s3_secret_key_set;
    let mut s3_force_path_style = form.s3_force_path_style;

    rsx! {
        label { class: "settings-field",
            span { "服务地址" }
            input {
                r#type: "text",
                value: "{s3_endpoint()}",
                oninput: move |event| s3_endpoint.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-field",
            span { "存储区域" }
            input {
                r#type: "text",
                value: "{s3_region()}",
                oninput: move |event| s3_region.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-field",
            span { "存储桶" }
            input {
                r#type: "text",
                value: "{s3_bucket()}",
                oninput: move |event| s3_bucket.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-field",
            span { "目录前缀（可选）" }
            input {
                r#type: "text",
                value: "{s3_prefix()}",
                oninput: move |event| s3_prefix.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-field",
            span { "访问密钥" }
            input {
                r#type: "text",
                value: "{s3_access_key()}",
                oninput: move |event| s3_access_key.set(event.value()),
                disabled,
            }
        }
        label { class: "settings-field",
            span {
                if s3_secret_key_set() {
                    "私有密钥（留空不修改）"
                } else {
                    "私有密钥"
                }
            }
            input {
                r#type: "password",
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
            span { "使用路径风格（MinIO 通常需要开启）" }
        }
    }
}
