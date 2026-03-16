use dioxus::prelude::*;

use super::super::state::SettingsFormState;
use super::forms::render_general_fields;
use super::shared::render_placeholder_section;
use super::storage::render_storage_fields;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    Account,
    General,
    Storage,
    Security,
    System,
    Maintenance,
    Users,
}

pub const ADMIN_SETTINGS_SECTIONS: [SettingsSection; 7] = [
    SettingsSection::General,
    SettingsSection::Storage,
    SettingsSection::Security,
    SettingsSection::Account,
    SettingsSection::System,
    SettingsSection::Maintenance,
    SettingsSection::Users,
];

pub const USER_SETTINGS_SECTIONS: [SettingsSection; 2] =
    [SettingsSection::Account, SettingsSection::Security];

impl SettingsSection {
    pub fn label(self) -> &'static str {
        match self {
            Self::Account => "账户",
            Self::General => "基础设置",
            Self::Storage => "存储设置",
            Self::Security => "账号安全",
            Self::System => "系统状态",
            Self::Maintenance => "维护工具",
            Self::Users => "用户与权限",
        }
    }

    pub fn title(self) -> &'static str {
        self.label()
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Account => "查看当前账户和会话状态。",
            Self::General => "管理站点名称和邮件配置。",
            Self::Storage => "配置本地目录或 S3。",
            Self::Security => "修改密码和登录安全。",
            Self::System => "查看健康与容量状态。",
            Self::Maintenance => "执行清理、备份和恢复。",
            Self::Users => "管理用户和权限。",
        }
    }

    pub fn uses_global_settings_actions(self) -> bool {
        matches!(self, Self::General | Self::Storage)
    }
}

pub fn render_settings_fields(
    form: SettingsFormState,
    disabled: bool,
    section: SettingsSection,
) -> Element {
    match section {
        SettingsSection::General => render_general_fields(form, disabled),
        SettingsSection::Storage => render_storage_fields(form, disabled),
        _ => render_placeholder_section(section),
    }
}
