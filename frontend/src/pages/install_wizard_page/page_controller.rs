use crate::pages::settings_page::{SettingsFormState, default_mail_link_base_url};
use crate::types::api::{AdminSettingsConfig, InstallBootstrapResponse};
use dioxus::prelude::*;

use super::summary::InstallWizardStep;
use super::summary::{
    initial_local_storage_path, initial_mail_smtp_port, initial_site_name, initial_storage_backend,
};

#[derive(Clone)]
pub(super) struct InstallWizardController {
    pub form: SettingsFormState,
    pub admin_email: Signal<String>,
    pub admin_password: Signal<String>,
    pub confirm_password: Signal<String>,
    pub show_admin_password: Signal<bool>,
    pub show_confirm_password: Signal<bool>,
    pub current_step: Signal<InstallWizardStep>,
}

pub(super) fn use_install_wizard_controller(
    initial_config: &AdminSettingsConfig,
    _on_installed: EventHandler<InstallBootstrapResponse>,
) -> InstallWizardController {
    let site_name = use_signal({
        let initial = initial_site_name(initial_config);
        move || initial.clone()
    });
    let storage_backend = use_signal({
        let initial = initial_storage_backend(initial_config);
        move || initial
    });
    let local_storage_path = use_signal({
        let initial = initial_local_storage_path(initial_config);
        move || initial.clone()
    });
    let mail_enabled = use_signal({
        let initial = initial_config.mail_enabled;
        move || initial
    });
    let mail_smtp_host = use_signal({
        let initial = initial_config.mail_smtp_host.clone();
        move || initial.clone()
    });
    let mail_smtp_port = use_signal({
        let initial = initial_mail_smtp_port(initial_config);
        move || initial.clone()
    });
    let mail_smtp_user = use_signal({
        let initial = initial_config.mail_smtp_user.clone().unwrap_or_default();
        move || initial.clone()
    });
    let mail_smtp_password = use_signal(String::new);
    let mail_smtp_password_set = use_signal({
        let initial = initial_config.mail_smtp_password_set;
        move || initial
    });
    let mail_from_email = use_signal({
        let initial = initial_config.mail_from_email.clone();
        move || initial.clone()
    });
    let mail_from_name = use_signal({
        let initial = initial_config.mail_from_name.clone();
        move || initial.clone()
    });
    let mail_link_base_url = use_signal({
        let initial = default_mail_link_base_url(&initial_config.mail_link_base_url);
        move || initial.clone()
    });

    let admin_email = use_signal(String::new);
    let admin_password = use_signal(String::new);
    let confirm_password = use_signal(String::new);
    let show_admin_password = use_signal(|| false);
    let show_confirm_password = use_signal(|| false);

    let current_step = use_signal(|| InstallWizardStep::Admin);
    let form = SettingsFormState {
        site_name,
        storage_backend,
        local_storage_path,
        mail_enabled,
        mail_smtp_host,
        mail_smtp_port,
        mail_smtp_user,
        mail_smtp_password,
        mail_smtp_password_set,
        mail_from_email,
        mail_from_name,
        mail_link_base_url,
    };

    InstallWizardController {
        form,
        admin_email,
        admin_password,
        confirm_password,
        show_admin_password,
        show_confirm_password,
        current_step,
    }
}
