use crate::store::SettingsAnchor;
use crate::types::api::{AdminSettingsConfig, StorageBackendKind, TestS3StorageConfigRequest};

use super::state::{SettingsFormState, display_mail_smtp_port};
use super::{S3ProviderPreset, SettingsSection};

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
    if !trimmed_option_eq((form.mail_smtp_user)(), config.mail_smtp_user.clone()) {
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
    if !trimmed_option_eq((form.s3_endpoint)(), config.s3_endpoint.clone()) {
        changes += 1;
    }
    if !trimmed_option_eq((form.s3_region)(), config.s3_region.clone()) {
        changes += 1;
    }
    if !trimmed_option_eq((form.s3_bucket)(), config.s3_bucket.clone()) {
        changes += 1;
    }
    if !trimmed_option_eq((form.s3_prefix)(), config.s3_prefix.clone()) {
        changes += 1;
    }
    if !trimmed_option_eq((form.s3_access_key)(), config.s3_access_key.clone()) {
        changes += 1;
    }
    if !(form.s3_secret_key)().trim().is_empty() {
        changes += 1;
    }
    if (form.s3_force_path_style)() != config.s3_force_path_style {
        changes += 1;
    }

    changes
}

pub(super) fn requires_s3_test_confirmation(
    form: SettingsFormState,
    config: &AdminSettingsConfig,
) -> bool {
    if !form.is_s3_backend() {
        return false;
    }

    if config.storage_backend != StorageBackendKind::S3 {
        return true;
    }

    has_s3_config_changes(form, config)
}

pub(super) fn has_s3_config_changes(form: SettingsFormState, config: &AdminSettingsConfig) -> bool {
    !trimmed_option_eq((form.s3_endpoint)(), config.s3_endpoint.clone())
        || !trimmed_option_eq((form.s3_region)(), config.s3_region.clone())
        || !trimmed_option_eq((form.s3_bucket)(), config.s3_bucket.clone())
        || !trimmed_option_eq((form.s3_prefix)(), config.s3_prefix.clone())
        || !trimmed_option_eq((form.s3_access_key)(), config.s3_access_key.clone())
        || !(form.s3_secret_key)().trim().is_empty()
        || (form.s3_force_path_style)() != config.s3_force_path_style
}

pub(super) fn is_current_s3_request_confirmed(
    form: SettingsFormState,
    last_tested_request: Option<TestS3StorageConfigRequest>,
) -> bool {
    last_tested_request.is_some_and(|tested| tested == form.build_s3_test_request())
}

pub(super) fn current_storage_summary(form: SettingsFormState) -> String {
    match (form.storage_backend)() {
        StorageBackendKind::Unknown => "未选择".to_string(),
        StorageBackendKind::Local => {
            let path = (form.local_storage_path)().trim().to_string();
            if path.is_empty() {
                "本地目录".to_string()
            } else {
                format!("本地目录 · {path}")
            }
        }
        StorageBackendKind::S3 => {
            let bucket = (form.s3_bucket)().trim().to_string();
            let prefix = (form.s3_prefix)().trim().trim_matches('/').to_string();
            let preset = (form.s3_provider_preset)();
            let provider_label = match preset {
                S3ProviderPreset::Other => "对象存储",
                _ => preset.label(),
            };

            if bucket.is_empty() {
                provider_label.to_string()
            } else if prefix.is_empty() {
                format!("{provider_label} · {bucket}")
            } else {
                format!("{provider_label} · {bucket}/{prefix}")
            }
        }
    }
}

pub(super) fn current_mail_summary(form: SettingsFormState) -> &'static str {
    if (form.mail_enabled)() {
        "已启用"
    } else {
        "未启用"
    }
}

fn trimmed_option_eq(draft: String, current: Option<String>) -> bool {
    let draft = draft.trim();
    let current = current.unwrap_or_default();
    draft == current.trim()
}

fn is_allowed_settings_section(is_admin: bool, section: SettingsSection) -> bool {
    if is_admin {
        true
    } else {
        matches!(
            section,
            SettingsSection::Account | SettingsSection::Security
        )
    }
}
