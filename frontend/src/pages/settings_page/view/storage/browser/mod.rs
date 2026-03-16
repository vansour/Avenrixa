mod loader;

use crate::app_context::{use_auth_store, use_settings_service, use_toast_store};
use crate::components::Modal;
use crate::types::api::StorageDirectoryEntry;
use dioxus::prelude::*;

use self::loader::{
    DEFAULT_SETTINGS_STORAGE_BROWSER_PATH, StorageBrowserSignals, load_settings_storage_directories,
};
use super::super::super::state::SettingsFormState;

#[component]
pub(super) fn LocalStoragePathPicker(form: SettingsFormState, disabled: bool) -> Element {
    let settings_service = use_settings_service();
    let auth_store = use_auth_store();
    let toast_store = use_toast_store();

    let mut local_storage_path = form.local_storage_path;
    let current_local_storage_path = local_storage_path();
    let requested_path = if current_local_storage_path.trim().is_empty() {
        DEFAULT_SETTINGS_STORAGE_BROWSER_PATH.to_string()
    } else {
        current_local_storage_path.clone()
    };

    let mut browser_open = use_signal(|| false);
    let browser_loading = use_signal(|| false);
    let browser_error = use_signal(String::new);
    let browser_current_path = use_signal(|| DEFAULT_SETTINGS_STORAGE_BROWSER_PATH.to_string());
    let browser_parent_path = use_signal(|| None::<String>);
    let browser_directories = use_signal(Vec::<StorageDirectoryEntry>::new);

    let open_browser_service = settings_service.clone();
    let browse_parent_service = settings_service.clone();
    let open_browser_auth_store = auth_store.clone();
    let browse_parent_auth_store = auth_store.clone();
    let open_browser_toast_store = toast_store.clone();
    let browse_parent_toast_store = toast_store.clone();
    let directory_entries = browser_directories();

    rsx! {
        div { class: "settings-field settings-field-full",
            span { "本地存储路径" }
            div { class: "install-path-picker",
                input {
                    class: "install-path-input",
                    r#type: "text",
                    value: "{current_local_storage_path}",
                    readonly: true,
                    disabled: true,
                }
                button {
                    class: "btn btn-ghost",
                    r#type: "button",
                    disabled: disabled || browser_loading(),
                    onclick: move |_| {
                        browser_open.set(true);
                        load_settings_storage_directories(
                            open_browser_service.clone(),
                            open_browser_auth_store.clone(),
                            open_browser_toast_store.clone(),
                            StorageBrowserSignals {
                                loading: browser_loading,
                                error: browser_error,
                                current_path: browser_current_path,
                                parent_path: browser_parent_path,
                                directories: browser_directories,
                            },
                            requested_path.clone(),
                        );
                    },
                    if browser_loading() { "读取中..." } else { "选择文件夹" }
                }
            }
            if browser_open() {
                Modal {
                    title: "选择本地存储目录".to_string(),
                    content_class: "storage-browser-modal-shell".to_string(),
                    on_close: move |_| browser_open.set(false),
                    div { class: "install-path-browser",
                        div { class: "install-path-browser-head",
                            code { class: "install-path-browser-current", "{browser_current_path()}" }
                            div { class: "install-path-browser-toolbar",
                                button {
                                    class: "btn btn-ghost",
                                    r#type: "button",
                                    disabled: disabled || browser_loading() || browser_parent_path().is_none(),
                                    onclick: move |_| {
                                        if let Some(parent_path) = browser_parent_path() {
                                            load_settings_storage_directories(
                                                browse_parent_service.clone(),
                                                browse_parent_auth_store.clone(),
                                                browse_parent_toast_store.clone(),
                                                StorageBrowserSignals {
                                                    loading: browser_loading,
                                                    error: browser_error,
                                                    current_path: browser_current_path,
                                                    parent_path: browser_parent_path,
                                                    directories: browser_directories,
                                                },
                                                parent_path,
                                            );
                                        }
                                    },
                                    "上一级"
                                }
                                button {
                                    class: "btn btn-primary",
                                    r#type: "button",
                                    disabled: disabled,
                                    onclick: move |_| {
                                        local_storage_path.set(browser_current_path());
                                        browser_open.set(false);
                                    },
                                    "选择当前文件夹"
                                }
                            }
                        }
                        div { class: "install-path-browser-panel",
                            if !browser_error().is_empty() {
                                p { class: "install-path-browser-error", "{browser_error()}" }
                            } else if browser_loading() {
                                p { class: "install-path-browser-empty", "正在读取目录..." }
                            } else if directory_entries.is_empty() {
                                p { class: "install-path-browser-empty", "当前目录下没有可继续展开的子目录。" }
                            } else {
                                div { class: "install-path-browser-list",
                                    {directory_entries.iter().map(|entry| {
                                        let browse_entry_service = settings_service.clone();
                                        let browse_entry_auth_store = auth_store.clone();
                                        let browse_entry_toast_store = toast_store.clone();
                                        let entry_path = entry.path.clone();
                                        let entry_name = entry.name.clone();
                                        rsx! {
                                            button {
                                                key: "{entry_path}",
                                                class: "install-path-browser-item",
                                                r#type: "button",
                                                disabled: disabled || browser_loading(),
                                                onclick: move |_| {
                                                    load_settings_storage_directories(
                                                        browse_entry_service.clone(),
                                                        browse_entry_auth_store.clone(),
                                                        browse_entry_toast_store.clone(),
                                                        StorageBrowserSignals {
                                                            loading: browser_loading,
                                                            error: browser_error,
                                                            current_path: browser_current_path,
                                                            parent_path: browser_parent_path,
                                                            directories: browser_directories,
                                                        },
                                                        entry_path.clone(),
                                                    );
                                                },
                                                span { class: "install-path-browser-folder" }
                                                span { class: "install-path-browser-name", "{entry_name}" }
                                            }
                                        }
                                    })}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
