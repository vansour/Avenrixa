use dioxus::prelude::*;

use super::super::super::super::state::SettingsFormState;

pub(super) fn render_mail_section(
    form: SettingsFormState,
    disabled: bool,
    compact: bool,
) -> Element {
    let mut mail_enabled = form.mail_enabled;
    let mut mail_smtp_host = form.mail_smtp_host;
    let mut mail_smtp_port = form.mail_smtp_port;
    let mut mail_smtp_user = form.mail_smtp_user;
    let mut mail_smtp_password = form.mail_smtp_password;
    let mail_smtp_password_set = form.mail_smtp_password_set;
    let mut mail_from_email = form.mail_from_email;
    let mut mail_from_name = form.mail_from_name;
    let mut mail_link_base_url = form.mail_link_base_url;
    let mail_is_enabled = mail_enabled();
    let section_class = if compact {
        "settings-subcard install-compact-subcard"
    } else {
        "settings-subcard"
    };
    let section_title = if compact {
        "邮件服务"
    } else {
        "邮件配置"
    };

    rsx! {
        div { class: section_class,
            h3 { "{section_title}" }
            div { class: "settings-grid",
                label { class: "settings-check settings-field-full",
                    input {
                        r#type: "checkbox",
                        checked: mail_enabled(),
                        onchange: move |event| mail_enabled.set(event.checked()),
                        disabled,
                    }
                    span { "启用邮件服务" }
                }

                if mail_is_enabled {
                    label { class: "settings-field",
                        span { "发件邮箱（必填）" }
                        input {
                            r#type: "email",
                            placeholder: "noreply@example.com",
                            value: "{mail_from_email()}",
                            oninput: move |event| mail_from_email.set(event.value()),
                            disabled,
                        }
                    }

                    label { class: "settings-field",
                        span { "发件人名称" }
                        input {
                            r#type: "text",
                            placeholder: "Avenrixa",
                            value: "{mail_from_name()}",
                            oninput: move |event| mail_from_name.set(event.value()),
                            disabled,
                        }
                    }

                    label { class: "settings-field settings-field-full",
                        span { "站点访问地址（必填）" }
                        input {
                            r#type: "url",
                            value: "{mail_link_base_url()}",
                            oninput: move |event| mail_link_base_url.set(event.value()),
                            placeholder: "https://img.example.com",
                            disabled,
                        }
                    }

                    label { class: "settings-field",
                        span { "SMTP 主机（必填）" }
                        input {
                            r#type: "text",
                            placeholder: "smtp.example.com",
                            value: "{mail_smtp_host()}",
                            oninput: move |event| mail_smtp_host.set(event.value()),
                            disabled,
                        }
                    }

                    label { class: "settings-field",
                        span { "SMTP 端口（必填）" }
                        input {
                            r#type: "number",
                            min: "1",
                            placeholder: "587",
                            value: "{mail_smtp_port()}",
                            oninput: move |event| mail_smtp_port.set(event.value()),
                            disabled,
                        }
                    }

                    label { class: "settings-field",
                        span { "SMTP 用户名" }
                        input {
                            r#type: "text",
                            placeholder: "可留空",
                            value: "{mail_smtp_user()}",
                            oninput: move |event| mail_smtp_user.set(event.value()),
                            disabled,
                        }
                    }

                    label { class: "settings-field",
                        span {
                            if mail_smtp_password_set() {
                                "SMTP 密码（留空不修改）"
                            } else {
                                "SMTP 密码"
                            }
                        }
                        input {
                            r#type: "password",
                            placeholder: if mail_smtp_password_set() {
                                "留空表示继续使用现有密码"
                            } else {
                                "输入 SMTP 密码"
                            },
                            value: "{mail_smtp_password()}",
                            oninput: move |event| mail_smtp_password.set(event.value()),
                            disabled,
                        }
                    }
                }
            }
        }
    }
}
