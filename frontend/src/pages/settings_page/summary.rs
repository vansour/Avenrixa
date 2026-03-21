use crate::store::SettingsAnchor;
use crate::types::api::AdminSettingsConfig;

use super::state::{SettingsFormState, display_mail_smtp_port};
use super::SettingsSection;

pub(super) fn resolved_settings_section(
    is_admin: bool,
    requested_section: Option<SettingsAnchor>,
) -> SettingsSection {
    match requested_section
        .map(settings_section_from_anchor)
        .filter(|section| is_allowed_settings_section(is_admin, *section))
    {
        Some(section) => section,
        None if is_admin => SettingsSection::General,
        None => SettingsSection::Account,
    }
}

pub(super) fn settings_section_from_anchor(anchor: SettingsAnchor) -> SettingsSection {
    match anchor {
        SettingsAnchor::Account => SettingsSection::Account,
        SettingsAnchor::General => SettingsSection::General,
        SettingsAnchor::Storage => SettingsSection::Storage,
        SettingsAnchor::Security => SettingsSection::Security,
        SettingsAnchor::System => SettingsSection::System,
        SettingsAnchor::Maintenance => SettingsSection::Maintenance,
        SettingsAnchor::Users => SettingsSection::Users,
    }
}

pub(super) fn count_config_changes(form: SettingsFormState, config: &AdminSettingsConfig) -> usize {
    let mut changes = 0;

    if (form.site_name)().trim() != config.site_name.trim() {
        changes += 1;
    }
    if (form.storage_backend)() != config.storage_backend {
        changes += 1;
    }
    if (form.local_storage_path)().trim() != config.local_storage_path.trim() {
        changes += 1;
    }
    if (form.mail_enabled)() != config.mail_enabled {
        changes += 1;
    }
    if (form.mail_smtp_host)().trim() != config.mail_smtp_host.trim() {
        changes += 1;
    }
    if (form.mail_smtp_port)().trim() != display_mail_smtp_port(config.mail_smtp_port) {
        changes += 1;
    }
    if !trimmed_option_eq(Some((form.mail_smtp_user)()), config.mail_smtp_user.clone()) {
        changes += 1;
    }
    if !(form.mail_smtp_password)().trim().is_empty() {
        changes += 1;
    }
    if (form.mail_from_email)().trim() != config.mail_from_email.trim() {
        changes += 1;
    }
    if (form.mail_from_name)().trim() != config.mail_from_name.trim() {
        changes += 1;
    }
    if (form.mail_link_base_url)().trim() != config.mail_link_base_url.trim() {
        changes += 1;
    }

    changes
}

fn trimmed_option_eq(left: Option<String>, right: Option<String>) -> bool {
    match (left.as_ref(), right.as_ref()) {
        (None, None) => true,
        (Some(l), Some(r)) => l == r,
        (Some(l), None) => l.trim().is_empty(),
        (None, Some(r)) => r.trim().is_empty(),
    }
}

fn is_allowed_settings_section(is_admin: bool, section: SettingsSection) -> bool {
    match section {
        SettingsSection::General => is_admin,
        SettingsSection::Storage => is_admin,
        SettingsSection::System => is_admin,
        SettingsSection::Security => is_admin,
        SettingsSection::Maintenance => is_admin,
        SettingsSection::Users => is_admin,
        SettingsSection::Account => true,
    }
}

pub(super) fn current_storage_summary(_form: SettingsFormState) -> String {
    "本地存储".to_string()
}

pub(super) fn current_mail_summary(form: SettingsFormState) -> String {
    if !(form.mail_enabled)() {
        "未启用".to_string()
    } else if install_mail_ready(form) {
        "已启用".to_string()
    } else {
        "待补全".to_string()
    }
}

fn install_mail_ready(form: SettingsFormState) -> bool {
    if !(form.mail_enabled)() {
        return true;
    }

    let smtp_host = (form.mail_smtp_host)().trim().to_string();
    let smtp_port = (form.mail_smtp_port)().trim().to_string();
    let smtp_user = (form.mail_smtp_user)().trim().to_string();
    let smtp_password = (form.mail_smtp_password)().trim().to_string();
    let from_email = (form.mail_from_email)().trim().to_string();
    let link_base_url = (form.mail_link_base_url)().trim().to_string();
    let password_ready =
        !smtp_password.is_empty() || ((form.mail_smtp_password_set)() && !smtp_user.is_empty());

    !smtp_host.is_empty()
        && !from_email.is_empty()
        && !link_base_url.is_empty()
        && smtp_port
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)
            .is_some()
        && (smtp_user.is_empty() == password_ready)
}

pub(super) fn settings_anchor_for_section(section: SettingsSection) -> SettingsAnchor {
    match section {
        SettingsSection::Account => SettingsAnchor::Account,
        SettingsSection::General => SettingsAnchor::General,
        SettingsSection::Storage => SettingsAnchor::Storage,
        SettingsSection::Security => SettingsAnchor::Security,
        SettingsSection::System => SettingsAnchor::System,
        SettingsSection::Maintenance => SettingsAnchor::Maintenance,
        SettingsSection::Users => SettingsAnchor::Users,
    }
}
