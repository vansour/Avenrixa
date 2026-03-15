use crate::components::ConfirmationTone;
use crate::types::api::UserRole;

use super::models::ConfirmationPlan;

pub(crate) fn role_change_confirmation_plan(
    email: &str,
    current_role: UserRole,
    next_role: UserRole,
) -> ConfirmationPlan {
    if current_role.is_admin() && next_role == UserRole::User {
        ConfirmationPlan {
            title: "降级管理员权限".to_string(),
            summary: format!("你正在把 {} 从管理员降级为普通用户。", email),
            consequences: vec![
                "该用户将失去系统设置、用户管理和维护工具访问权限。".to_string(),
                "如果这是最后一个管理员，后续将无法再通过界面管理系统。".to_string(),
                "建议先确认仍有其他管理员账户可用。".to_string(),
            ],
            confirm_label: "确认降级".to_string(),
            cancel_label: "取消".to_string(),
            tone: ConfirmationTone::Danger,
            confirm_phrase: Some(email.to_string()),
            confirm_hint: Some(format!("请输入 {} 以确认降级", email)),
        }
    } else {
        ConfirmationPlan {
            title: "提升用户权限".to_string(),
            summary: format!("你正在把 {} 提升为管理员。", email),
            consequences: vec![
                "该用户将获得系统设置、用户管理和维护工具访问权限。".to_string(),
                "管理员可以修改底层配置并执行高风险维护操作。".to_string(),
            ],
            confirm_label: "确认提升".to_string(),
            cancel_label: "取消".to_string(),
            tone: ConfirmationTone::Warning,
            confirm_phrase: None,
            confirm_hint: None,
        }
    }
}
