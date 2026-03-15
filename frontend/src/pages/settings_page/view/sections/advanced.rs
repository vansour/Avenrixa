use crate::types::api::Setting;
use dioxus::prelude::*;
use std::collections::HashMap;

use super::super::shared::textarea_rows;

#[component]
pub fn AdvancedSettingsSection(
    settings: Vec<Setting>,
    setting_drafts: Signal<HashMap<String, String>>,
    error_message: String,
    success_message: String,
    is_loading: bool,
    saving_key: Option<String>,
    #[props(default)] on_refresh: EventHandler<MouseEvent>,
    #[props(default)] on_save_setting: EventHandler<String>,
) -> Element {
    let is_saving_any = saving_key.is_some();

    rsx! {
        div { class: "settings-stack",
            div { class: "settings-list-toolbar",
                div { class: "settings-toolbar-meta",
                    span { class: "stat-pill", "键值 {settings.len()} 项" }
                }
                div { class: "settings-inline-actions",
                    button {
                        class: "btn",
                        disabled: is_loading || is_saving_any,
                        onclick: move |event| on_refresh.call(event),
                        if is_loading { "刷新中..." } else { "刷新键值" }
                    }
                }
            }

            if !error_message.is_empty() {
                div { class: "error-banner", "{error_message}" }
            }

            if !success_message.is_empty() {
                div { class: "settings-banner settings-banner-success", "{success_message}" }
            }

            div { class: "settings-banner settings-banner-neutral",
                "原始键值会直接写入底层 settings 表。标记为“需确认”的条目会触发分级确认，涉及存储切换时还会要求输入设置键名。"
            }

            if is_loading && settings.is_empty() {
                div { class: "settings-placeholder settings-placeholder-compact",
                    h3 { "正在加载原始设置" }
                }
            } else if settings.is_empty() {
                div { class: "settings-placeholder settings-placeholder-compact",
                    h3 { "暂时没有原始键值" }
                }
            } else {
                div { class: "settings-kv-list",
                    {settings.into_iter().map(|setting| {
                        let setting_key_for_input = setting.key.clone();
                        let setting_key_for_save = setting.key.clone();
                        let draft_value = setting_drafts()
                            .get(&setting.key)
                            .cloned()
                            .unwrap_or_else(|| setting.value.clone());
                        let is_row_saving = saving_key.as_deref() == Some(setting.key.as_str());
                        let is_readonly = !setting.editable;
                        rsx! {
                            article { class: "settings-kv-card",
                                div { class: "settings-kv-head",
                                    div { class: "settings-kv-copy",
                                        h3 { class: "settings-kv-key", "{setting.key}" }
                                        div { class: "settings-kv-badges",
                                            if is_readonly {
                                                span { class: "settings-kv-badge", "只读" }
                                            }
                                            if setting.masked {
                                                span { class: "settings-kv-badge is-warning", "已脱敏" }
                                            }
                                            if setting.requires_confirmation {
                                                span { class: "settings-kv-badge is-warning", "需二次确认" }
                                            }
                                        }
                                    }
                                    button {
                                        class: "btn btn-primary",
                                        disabled: is_loading || is_saving_any || is_readonly,
                                        onclick: move |_| on_save_setting.call(setting_key_for_save.clone()),
                                        if is_readonly {
                                            "受保护"
                                        } else if is_row_saving {
                                            "保存中..."
                                        } else {
                                            "保存键值"
                                        }
                                    }
                                }

                                textarea {
                                    class: "settings-kv-input",
                                    value: "{draft_value}",
                                    rows: "{textarea_rows(&draft_value)}",
                                    disabled: is_loading || is_saving_any || is_readonly,
                                    oninput: move |event| {
                                        let mut drafts = setting_drafts();
                                        drafts.insert(setting_key_for_input.clone(), event.value());
                                        setting_drafts.set(drafts);
                                    },
                                }
                            }
                        }
                    })}
                }
            }
        }
    }
}
