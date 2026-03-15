use dioxus::prelude::*;

pub(super) fn render_site_name_section(
    mut site_name: Signal<String>,
    disabled: bool,
    compact: bool,
) -> Element {
    let section_class = if compact {
        "settings-subcard install-compact-subcard"
    } else {
        "settings-subcard"
    };
    let section_title = if compact {
        "站点信息"
    } else {
        "站点识别"
    };

    rsx! {
        div { class: section_class,
            h3 { "{section_title}" }
            div { class: "settings-grid",
                label { class: "settings-field settings-field-full",
                    span { "网站名称" }
                    input {
                        r#type: "text",
                        value: "{site_name()}",
                        oninput: move |event| site_name.set(event.value()),
                        disabled,
                    }
                }
            }
        }
    }
}
