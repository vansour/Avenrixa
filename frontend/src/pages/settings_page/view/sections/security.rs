use dioxus::prelude::*;

#[component]
pub fn SecuritySettingsSection(
    current_password: Signal<String>,
    new_password: Signal<String>,
    confirm_password: Signal<String>,
    error_message: String,
    success_message: String,
    is_submitting: bool,
    #[props(default)] on_submit: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "settings-stack",
            if !error_message.is_empty() {
                div { class: "error-banner", "{error_message}" }
            }

            if !success_message.is_empty() {
                div { class: "settings-banner settings-banner-success", "{success_message}" }
            }

            div { class: "settings-grid settings-grid-single",
                label { class: "settings-field settings-field-full",
                    span { "当前密码" }
                    input {
                        r#type: "password",
                        value: "{current_password()}",
                        oninput: move |event| current_password.set(event.value()),
                        disabled: is_submitting,
                    }
                }

                label { class: "settings-field settings-field-full",
                    span { "新密码" }
                    input {
                        r#type: "password",
                        value: "{new_password()}",
                        oninput: move |event| new_password.set(event.value()),
                        disabled: is_submitting,
                    }
                }

                label { class: "settings-field settings-field-full",
                    span { "确认新密码" }
                    input {
                        r#type: "password",
                        value: "{confirm_password()}",
                        oninput: move |event| confirm_password.set(event.value()),
                        disabled: is_submitting,
                    }
                }
            }

            div { class: "settings-actions",
                button {
                    class: "btn btn-primary",
                    disabled: is_submitting,
                    onclick: move |event| on_submit.call(event),
                    if is_submitting { "修改中..." } else { "修改密码" }
                }
            }
        }
    }
}
