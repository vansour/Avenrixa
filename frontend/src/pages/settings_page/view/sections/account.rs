use crate::types::api::UserRole;
use dioxus::prelude::*;

use super::super::shared::render_metric_card;

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
            div { class: "settings-metric-grid",
                {render_metric_card("邮箱", email)}
                {render_metric_card("角色", role.label().to_string())}
                {render_metric_card("创建时间", created_at)}
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
