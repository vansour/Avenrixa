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
    Audit,
    Advanced,
}

pub const ADMIN_SETTINGS_SECTIONS: [SettingsSection; 9] = [
    SettingsSection::General,
    SettingsSection::Storage,
    SettingsSection::Security,
    SettingsSection::Account,
    SettingsSection::System,
    SettingsSection::Maintenance,
    SettingsSection::Users,
    SettingsSection::Audit,
    SettingsSection::Advanced,
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
            Self::Audit => "审计日志",
            Self::Advanced => "高级设置",
        }
    }

    pub fn title(self) -> &'static str {
        self.label()
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Account => "查看当前登录账户信息，并管理当前会话。",
            Self::General => "维护站点名称、邮件发送和验证链接等基础配置。",
            Self::Storage => "配置图片写入位置，并决定使用本地目录还是对象存储。",
            Self::Security => "修改当前账户密码，收紧登录安全。",
            Self::System => "查看健康检查、容量和整体运行状态。",
            Self::Maintenance => "执行清理与备份等高风险维护操作。",
            Self::Users => "查看账户列表，并调整用户权限。",
            Self::Audit => "追踪关键管理操作与系统事件。",
            Self::Advanced => "处理底层键值配置，仅在明确知道影响时修改。",
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
