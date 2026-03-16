use crate::types::api::UserRole;
use dioxus::prelude::*;

#[component]
pub fn AccountSettingsSection(
    email: String,
    role: UserRole,
    created_at: String,
    is_logging_out: bool,
    #[props(default)] on_logout: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "settings-stack",
            div { class: "settings-grid",
                label { class: "settings-field settings-field-full",
                    span { "账户邮箱" }
                    input {
                        r#type: "text",
                        value: "{email}",
                        readonly: true,
                        class: "settings-readonly-input",
                    }
                }

                label { class: "settings-field",
                    span { "角色" }
                    input {
                        r#type: "text",
                        value: "{role.label()}",
                        readonly: true,
                        class: "settings-readonly-input",
                    }
                }

                label { class: "settings-field",
                    span { "创建时间" }
                    input {
                        r#type: "text",
                        value: "{created_at}",
                        readonly: true,
                        class: "settings-readonly-input",
                    }
                }
            }

            div { class: "settings-action-grid",
                article { class: "settings-action-card settings-action-card-danger",
                    div { class: "settings-action-copy",
                        h3 { "退出登录" }
                    }
                    button {
                        class: "btn btn-danger",
                        disabled: is_logging_out,
                        onclick: move |event| on_logout.call(event),
                        if is_logging_out { "退出中..." } else { "退出登录" }
                    }
                }
            }
        }
    }
}
