mod mail;
mod site;

use dioxus::prelude::*;

use self::mail::render_mail_section;
use self::site::render_site_name_section;
use super::super::super::state::SettingsFormState;

pub fn render_general_fields(form: SettingsFormState, disabled: bool) -> Element {
    let site_name = form.site_name;

    rsx! {
        div { class: "settings-stack",
            {render_site_name_section(site_name, disabled, false)}
            {render_mail_section(form, disabled, false)}
        }
    }
}

pub fn render_general_fields_compact(form: SettingsFormState, disabled: bool) -> Element {
    let site_name = form.site_name;

    rsx! {
        div { class: "settings-stack",
            {render_site_name_section(site_name, disabled, true)}
            {render_mail_section(form, disabled, true)}
        }
    }
}
