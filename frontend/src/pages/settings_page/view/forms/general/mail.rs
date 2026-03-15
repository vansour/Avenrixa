use dioxus::prelude::*;

use super::super::super::super::state::SettingsFormState;

pub(super) fn render_mail_status_banner(mail_is_enabled: bool) -> Element {
    rsx! {
        if mail_is_enabled {
            div { class: "settings-banner settings-banner-neutral",
                "邮件服务已开启。公开注册、邮箱验证和密码找回都会依赖这里的 SMTP 与跳转地址配置。"
            }
        } else {
            div { class: "settings-banner settings-banner-neutral",
                "邮件服务当前关闭。用户仍可登录，但公开注册后的邮箱验证和密码找回邮件不会发送。"
            }
        }
    }
}

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
        "邮件投递"
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
                        span { "发件邮箱" }
                        input {
                            r#type: "email",
                            value: "{mail_from_email()}",
                            oninput: move |event| mail_from_email.set(event.value()),
                            disabled,
                        }
                    }

                    label { class: "settings-field",
                        span { "发件人名称" }
                        input {
                            r#type: "text",
                            value: "{mail_from_name()}",
                            oninput: move |event| mail_from_name.set(event.value()),
                            disabled,
                        }
                    }

                    label { class: "settings-field settings-field-full",
                        span { "站点访问地址（用于邮件链接）" }
                        input {
                            r#type: "url",
                            value: "{mail_link_base_url()}",
                            oninput: move |event| mail_link_base_url.set(event.value()),
                            placeholder: "https://img.example.com",
                            disabled,
                        }
                        small { class: "settings-field-hint",
                            "用户点击邮件里的验证或重置链接后，会回到这里。"
                        }
                    }

                    label { class: "settings-field",
                        span { "SMTP 主机" }
                        input {
                            r#type: "text",
                            value: "{mail_smtp_host()}",
                            oninput: move |event| mail_smtp_host.set(event.value()),
                            disabled,
                        }
                    }

                    label { class: "settings-field",
                        span { "SMTP 端口" }
                        input {
                            r#type: "number",
                            min: "1",
                            value: "{mail_smtp_port()}",
                            oninput: move |event| mail_smtp_port.set(event.value()),
                            disabled,
                        }
                    }

                    label { class: "settings-field",
                        span { "SMTP 用户名" }
                        input {
                            r#type: "text",
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
