mod mail;
mod site;

use dioxus::prelude::*;

use self::mail::{render_mail_section, render_mail_status_banner};
use self::site::render_site_name_section;
use super::super::super::state::SettingsFormState;
use super::super::shared::{render_metric_card, summary_value};

pub fn render_general_fields(form: SettingsFormState, disabled: bool) -> Element {
    let site_name = form.site_name;
    let mail_enabled = form.mail_enabled;
    let mail_link_base_url = form.mail_link_base_url;
    let mail_is_enabled = mail_enabled();
    let mail_jump_target = if mail_is_enabled {
        summary_value(mail_link_base_url())
    } else {
        "未启用".to_string()
    };

    rsx! {
        div { class: "settings-stack",
            div { class: "settings-status-summary",
                {render_metric_card("站点名称", summary_value(site_name()))}
                {render_metric_card("邮件服务", if mail_is_enabled { "已启用".to_string() } else { "未启用".to_string() })}
                {render_metric_card("站点访问地址", mail_jump_target)}
            }

            {render_mail_status_banner(mail_is_enabled)}
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
