mod browser;

use crate::types::api::StorageBackendKind;
use dioxus::prelude::*;

use self::browser::LocalStoragePathPicker;
use super::super::state::SettingsFormState;

pub fn render_storage_fields(form: SettingsFormState, disabled: bool) -> Element {
    let mut storage_backend = form.storage_backend;
    let selected_backend = (form.storage_backend)();

    rsx! {
        div { class: "settings-stack",
            div { class: "settings-subcard",
                h3 { "存储后端" }
                div { class: "settings-grid settings-grid-single",
                    label { class: "settings-field",
                        span { "存储后端" }
                        select {
                            value: "{selected_backend.as_str()}",
                            onchange: move |event| {
                                storage_backend.set(StorageBackendKind::parse(&event.value()))
                            },
                            disabled,
                            option { value: StorageBackendKind::Unknown.as_str(), "请选择存储后端" }
                            option { value: StorageBackendKind::Local.as_str(), "本地存储" }
                        }
                    }
                }
            }

            if selected_backend == StorageBackendKind::Local {
                div { class: "settings-subcard",
                    h3 { "本地存储" }
                    div { class: "settings-grid settings-grid-single",
                        LocalStoragePathPicker { form, disabled }
                    }
                }
            }
        }
    }
}

pub fn render_storage_fields_compact(form: SettingsFormState, disabled: bool) -> Element {
    let mut storage_backend = form.storage_backend;
    let selected_backend = (form.storage_backend)();

    rsx! {
        div { class: "settings-stack",
            div { class: "settings-subcard install-compact-subcard",
                h3 { "存储后端" }
                div { class: "settings-grid settings-grid-single",
                    label { class: "settings-field",
                        span { "存储后端" }
                        select {
                            value: "{selected_backend.as_str()}",
                            onchange: move |event| {
                                storage_backend.set(StorageBackendKind::parse(&event.value()))
                            },
                            disabled,
                            option { value: StorageBackendKind::Unknown.as_str(), "请选择存储后端" }
                            option { value: StorageBackendKind::Local.as_str(), "本地存储" }
                        }
                    }
                }
            }

            if selected_backend == StorageBackendKind::Local {
                div { class: "settings-subcard install-compact-subcard",
                    h3 { "本地存储" }
                    div { class: "settings-grid settings-grid-single",
                        LocalStoragePathPicker { form, disabled }
                    }
                }
            }
        }
    }
}
