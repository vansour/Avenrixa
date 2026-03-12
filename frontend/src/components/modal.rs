use dioxus::prelude::*;

/// Modal 对话框组件
#[component]
pub fn Modal(
    #[props(default)] title: String,
    #[props(default)] content_class: String,
    children: Element,
    on_close: EventHandler<()>,
) -> Element {
    let content_class = if content_class.trim().is_empty() {
        "modal-content".to_string()
    } else {
        format!("modal-content {}", content_class.trim())
    };

    rsx! {
        div { class: "modal-overlay",
            div { class: "{content_class}",
                h2 { class: "modal-title", "{title}" }
                div { class: "modal-close", onclick: move |_| on_close.call(()), "×" }
                {children}
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationTone {
    Warning,
    Danger,
}

impl ConfirmationTone {
    fn badge_label(self) -> &'static str {
        match self {
            Self::Warning => "二次确认",
            Self::Danger => "高风险操作",
        }
    }

    fn surface_class(self) -> &'static str {
        match self {
            Self::Warning => "is-warning",
            Self::Danger => "is-danger",
        }
    }
}

#[component]
pub fn ConfirmationModal(
    title: String,
    summary: String,
    consequences: Vec<String>,
    confirm_label: String,
    cancel_label: String,
    tone: ConfirmationTone,
    #[props(default)] confirm_phrase: Option<String>,
    #[props(default)] confirm_hint: Option<String>,
    #[props(default)] is_submitting: bool,
    on_close: EventHandler<()>,
    on_confirm: EventHandler<()>,
) -> Element {
    let mut typed_phrase = use_signal(String::new);
    let phrase_ready = confirm_phrase
        .as_ref()
        .is_none_or(|phrase| typed_phrase().trim() == phrase.trim());
    let phrase_requirement = confirm_phrase.clone().map(|phrase| {
        let hint = confirm_hint
            .clone()
            .unwrap_or_else(|| format!("请输入 {}", phrase));
        (phrase, hint)
    });

    rsx! {
        Modal {
            title,
            on_close,
            div { class: format!("confirm-dialog {}", tone.surface_class()),
                div { class: "confirm-head",
                    span { class: format!("confirm-badge {}", tone.surface_class()), "{tone.badge_label()}" }
                    p { class: "confirm-summary", "{summary}" }
                }

                if !consequences.is_empty() {
                    div { class: "confirm-section",
                        p { class: "confirm-section-title", "执行后会发生：" }
                        ul { class: "confirm-impact-list",
                            for item in consequences {
                                li { "{item}" }
                            }
                        }
                    }
                }

                if let Some((_phrase, hint)) = phrase_requirement {
                    div { class: "confirm-section",
                        p { class: "confirm-section-title", "输入确认词后才能继续" }
                        label { class: "settings-field confirm-field",
                            span { "{hint}" }
                            input {
                                r#type: "text",
                                value: "{typed_phrase()}",
                                disabled: is_submitting,
                                oninput: move |event| typed_phrase.set(event.value()),
                            }
                        }
                    }
                }

                div { class: "confirm-actions",
                    button {
                        class: "btn btn-ghost",
                        disabled: is_submitting,
                        onclick: move |_| on_close.call(()),
                        "{cancel_label}"
                    }
                    button {
                        class: if matches!(tone, ConfirmationTone::Danger) {
                            "btn btn-danger"
                        } else {
                            "btn btn-primary"
                        },
                        disabled: is_submitting || !phrase_ready,
                        onclick: move |_| on_confirm.call(()),
                        if is_submitting {
                            "执行中..."
                        } else {
                            "{confirm_label}"
                        }
                    }
                }
            }
        }
    }
}
