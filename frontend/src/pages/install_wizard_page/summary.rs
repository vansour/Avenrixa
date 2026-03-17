use crate::pages::settings_page::SettingsFormState;
use crate::types::api::{AdminSettingsConfig, StorageBackendKind, TestS3StorageConfigRequest};

const MIN_ADMIN_PASSWORD_LENGTH: usize = 12;
const DEFAULT_INSTALL_SITE_NAME: &str = "Avenrixa";
pub(super) const DEFAULT_INSTALL_STORAGE_BROWSER_PATH: &str = "/";
pub(super) const INSTALL_WIZARD_STEPS: [InstallWizardStep; 4] = [
    InstallWizardStep::Admin,
    InstallWizardStep::General,
    InstallWizardStep::Storage,
    InstallWizardStep::Review,
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum S3TestFeedbackTone {
    Neutral,
    Success,
    Error,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum InstallWizardStep {
    Admin,
    General,
    Storage,
    Review,
}

impl InstallWizardStep {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Admin => "创建管理员账号",
            Self::General => "配置站点信息",
            Self::Storage => "确认存储方案",
            Self::Review => "检查并初始化",
        }
    }

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Admin => "第 1 步：创建管理员账号",
            Self::General => "第 2 步：配置站点信息",
            Self::Storage => "第 3 步：确认存储方案",
            Self::Review => "第 4 步：检查并初始化",
        }
    }
}

pub(super) fn initial_local_storage_path(config: &AdminSettingsConfig) -> String {
    config.local_storage_path.trim().to_string()
}

pub(super) fn initial_site_name(config: &AdminSettingsConfig) -> String {
    let site_name = config.site_name.trim();
    if site_name == DEFAULT_INSTALL_SITE_NAME {
        String::new()
    } else {
        site_name.to_string()
    }
}

pub(super) fn initial_storage_backend(config: &AdminSettingsConfig) -> StorageBackendKind {
    let has_local_path = !config.local_storage_path.trim().is_empty();
    let has_s3_config = config
        .s3_endpoint
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty())
        || config
            .s3_region
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
        || config
            .s3_bucket
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
        || config
            .s3_access_key
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
        || config.s3_secret_key_set;

    if !has_local_path && !has_s3_config {
        StorageBackendKind::Unknown
    } else {
        config.storage_backend
    }
}

pub(super) fn initial_mail_smtp_port(config: &AdminSettingsConfig) -> String {
    if config.mail_smtp_port == 0 {
        String::new()
    } else {
        config.mail_smtp_port.to_string()
    }
}

pub(super) fn install_step_index(step: InstallWizardStep) -> usize {
    INSTALL_WIZARD_STEPS
        .iter()
        .position(|candidate| *candidate == step)
        .unwrap_or(0)
}

pub(super) fn install_step_state_class(
    step: InstallWizardStep,
    current_step: InstallWizardStep,
    admin_ready: bool,
    general_ready: bool,
    storage_ready: bool,
    review_ready: bool,
) -> &'static str {
    if step == current_step {
        "is-active"
    } else if install_step_complete(
        step,
        admin_ready,
        general_ready,
        storage_ready,
        review_ready,
    ) {
        "is-done"
    } else {
        "is-pending"
    }
}

pub(super) fn install_step_state_text(
    step: InstallWizardStep,
    current_step: InstallWizardStep,
    admin_ready: bool,
    general_ready: bool,
    storage_ready: bool,
    review_ready: bool,
) -> &'static str {
    if step == current_step {
        "进行中"
    } else if install_step_complete(
        step,
        admin_ready,
        general_ready,
        storage_ready,
        review_ready,
    ) {
        "已完成"
    } else {
        "待完成"
    }
}

pub(super) fn install_step_index_badge(
    step: InstallWizardStep,
    index: usize,
    current_step: InstallWizardStep,
    admin_ready: bool,
    general_ready: bool,
    storage_ready: bool,
    review_ready: bool,
) -> String {
    if step != current_step
        && install_step_complete(
            step,
            admin_ready,
            general_ready,
            storage_ready,
            review_ready,
        )
    {
        "✓".to_string()
    } else {
        index.to_string()
    }
}

pub(super) fn install_step_complete(
    step: InstallWizardStep,
    admin_ready: bool,
    general_ready: bool,
    storage_ready: bool,
    review_ready: bool,
) -> bool {
    match step {
        InstallWizardStep::Admin => admin_ready,
        InstallWizardStep::General => general_ready,
        InstallWizardStep::Storage => storage_ready,
        InstallWizardStep::Review => review_ready,
    }
}

pub(super) fn install_step_accessible(
    step: InstallWizardStep,
    admin_ready: bool,
    general_ready: bool,
    storage_ready: bool,
) -> bool {
    match step {
        InstallWizardStep::Admin => true,
        InstallWizardStep::General => admin_ready,
        InstallWizardStep::Storage => admin_ready && general_ready,
        InstallWizardStep::Review => admin_ready && general_ready && storage_ready,
    }
}

pub(super) fn install_admin_email_error(email: &str) -> Option<String> {
    let email = email.trim();
    if email.is_empty() || email.contains('@') {
        None
    } else {
        Some("请输入有效的管理员邮箱".to_string())
    }
}

pub(super) fn install_admin_password_error(password: &str) -> Option<String> {
    let password = password.trim();
    if password.is_empty() || password.len() >= MIN_ADMIN_PASSWORD_LENGTH {
        None
    } else {
        Some(format!(
            "管理员密码至少需要 {} 个字符",
            MIN_ADMIN_PASSWORD_LENGTH
        ))
    }
}

pub(super) fn install_admin_confirm_password_error(
    password: &str,
    confirm: &str,
) -> Option<String> {
    if password.is_empty() || confirm.is_empty() || password == confirm {
        None
    } else {
        Some("两次输入的管理员密码不一致".to_string())
    }
}

pub(super) fn install_admin_submit_error(
    email: &str,
    password: &str,
    confirm: &str,
) -> Option<String> {
    let email = email.trim();
    if email.is_empty() {
        Some("请填写管理员邮箱".to_string())
    } else if password.trim().is_empty() {
        Some("请填写管理员密码".to_string())
    } else if password.trim().len() < MIN_ADMIN_PASSWORD_LENGTH {
        Some(format!(
            "管理员密码至少需要 {} 个字符",
            MIN_ADMIN_PASSWORD_LENGTH
        ))
    } else if password != confirm {
        Some("两次输入的管理员密码不一致".to_string())
    } else {
        None
    }
}

pub(super) fn install_admin_ready(email: &str, password: &str, confirm: &str) -> bool {
    email.trim().contains('@') && install_admin_submit_error(email, password, confirm).is_none()
}

pub(super) fn install_mail_ready(form: SettingsFormState) -> bool {
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

pub(super) fn install_storage_ready(form: SettingsFormState, s3_test_confirmed: bool) -> bool {
    match (form.storage_backend)() {
        StorageBackendKind::Unknown => false,
        StorageBackendKind::Local => !(form.local_storage_path)().trim().is_empty(),
        StorageBackendKind::S3 => form.is_s3_configuration_complete() && s3_test_confirmed,
    }
}

pub(super) fn is_current_install_s3_request_confirmed(
    form: SettingsFormState,
    last_tested_request: Option<TestS3StorageConfigRequest>,
) -> bool {
    last_tested_request.is_some_and(|tested| tested == form.build_s3_test_request())
}

pub(super) fn current_install_storage_summary(form: SettingsFormState) -> String {
    match (form.storage_backend)() {
        StorageBackendKind::Local => "本地存储".to_string(),
        StorageBackendKind::S3 => "对象存储（兼容 S3）".to_string(),
        StorageBackendKind::Unknown => "待选择".to_string(),
    }
}

pub(super) fn current_install_mail_summary(form: SettingsFormState) -> String {
    if !(form.mail_enabled)() {
        "未启用".to_string()
    } else if install_mail_ready(form) {
        "已启用".to_string()
    } else {
        "待补全".to_string()
    }
}

pub(super) fn summary_or_pending(value: String) -> String {
    let value = value.trim().to_string();
    if value.is_empty() {
        "待填写".to_string()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_admin_password_error_is_shown_only_for_short_non_empty_passwords() {
        assert_eq!(install_admin_password_error(""), None);
        assert_eq!(
            install_admin_password_error("short"),
            Some("管理员密码至少需要 12 个字符".to_string())
        );
        assert_eq!(install_admin_password_error("Password123!"), None);
    }

    #[test]
    fn install_admin_confirm_password_error_waits_for_both_values_and_checks_match() {
        assert_eq!(
            install_admin_confirm_password_error("", "Password123!"),
            None
        );
        assert_eq!(
            install_admin_confirm_password_error("Password123!", ""),
            None
        );
        assert_eq!(
            install_admin_confirm_password_error("Password123!", "Password321!"),
            Some("两次输入的管理员密码不一致".to_string())
        );
        assert_eq!(
            install_admin_confirm_password_error("Password123!", "Password123!"),
            None
        );
    }

    #[test]
    fn install_admin_submit_error_matches_install_requirements() {
        assert_eq!(
            install_admin_submit_error("", "Password123!", "Password123!"),
            Some("请填写管理员邮箱".to_string())
        );
        assert_eq!(
            install_admin_submit_error("admin@example.com", "", ""),
            Some("请填写管理员密码".to_string())
        );
        assert_eq!(
            install_admin_submit_error("admin@example.com", "short", "short"),
            Some("管理员密码至少需要 12 个字符".to_string())
        );
        assert_eq!(
            install_admin_submit_error("admin@example.com", "Password123!", "Password321!"),
            Some("两次输入的管理员密码不一致".to_string())
        );
        assert_eq!(
            install_admin_submit_error("admin@example.com", "Password123!", "Password123!"),
            None
        );
    }

    #[test]
    fn install_admin_ready_requires_valid_email_and_matching_strong_password() {
        assert!(!install_admin_ready(
            "invalid-email",
            "Password123!",
            "Password123!"
        ));
        assert!(!install_admin_ready("admin@example.com", "short", "short"));
        assert!(install_admin_ready(
            "admin@example.com",
            "Password123!",
            "Password123!"
        ));
    }

    #[test]
    fn initial_local_storage_path_keeps_empty_value_until_user_selects_directory() {
        let config = AdminSettingsConfig {
            site_name: "Avenrixa".to_string(),
            storage_backend: StorageBackendKind::Local,
            local_storage_path: "   ".to_string(),
            mail_enabled: false,
            mail_smtp_host: String::new(),
            mail_smtp_port: 587,
            mail_smtp_user: None,
            mail_smtp_password_set: false,
            mail_from_email: String::new(),
            mail_from_name: String::new(),
            mail_link_base_url: String::new(),
            s3_endpoint: None,
            s3_region: None,
            s3_bucket: None,
            s3_prefix: None,
            s3_access_key: None,
            s3_secret_key_set: false,
            s3_force_path_style: true,
            restart_required: false,
            settings_version: String::new(),
        };

        assert!(initial_local_storage_path(&config).is_empty());
    }

    #[test]
    fn initial_site_name_clears_default_brand_name() {
        let default_config = AdminSettingsConfig {
            site_name: "Avenrixa".to_string(),
            storage_backend: StorageBackendKind::Local,
            local_storage_path: String::new(),
            mail_enabled: false,
            mail_smtp_host: String::new(),
            mail_smtp_port: 587,
            mail_smtp_user: None,
            mail_smtp_password_set: false,
            mail_from_email: String::new(),
            mail_from_name: String::new(),
            mail_link_base_url: String::new(),
            s3_endpoint: None,
            s3_region: None,
            s3_bucket: None,
            s3_prefix: None,
            s3_access_key: None,
            s3_secret_key_set: false,
            s3_force_path_style: true,
            restart_required: false,
            settings_version: String::new(),
        };
        let custom_config = AdminSettingsConfig {
            site_name: "Acme Images".to_string(),
            ..default_config.clone()
        };

        assert!(initial_site_name(&default_config).is_empty());
        assert_eq!(initial_site_name(&custom_config), "Acme Images".to_string());
    }

    #[test]
    fn install_step_accessible_requires_previous_steps_to_finish() {
        assert!(install_step_accessible(
            InstallWizardStep::Admin,
            false,
            false,
            false
        ));
        assert!(!install_step_accessible(
            InstallWizardStep::General,
            false,
            false,
            false
        ));
        assert!(!install_step_accessible(
            InstallWizardStep::Storage,
            true,
            false,
            false
        ));
        assert!(install_step_accessible(
            InstallWizardStep::Review,
            true,
            true,
            true
        ));
    }
}
