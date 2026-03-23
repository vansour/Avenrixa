use dioxus::prelude::*;

use crate::types::api::StorageBackendKind;

use super::super::state::SettingsFormState;

pub(super) fn build_settings_form() -> SettingsFormState {
    SettingsFormState {
        site_name: use_signal(String::new),
        storage_backend: use_signal(|| StorageBackendKind::Unknown),
        local_storage_path: use_signal(String::new),
        mail_enabled: use_signal(|| false),
        mail_smtp_host: use_signal(String::new),
        mail_smtp_port: use_signal(String::new),
        mail_smtp_user: use_signal(String::new),
        mail_smtp_password: use_signal(String::new),
        mail_smtp_password_set: use_signal(|| false),
        mail_from_email: use_signal(String::new),
        mail_from_name: use_signal(String::new),
        mail_link_base_url: use_signal(String::new),
    }
}
