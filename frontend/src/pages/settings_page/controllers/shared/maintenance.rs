use crate::components::ConfirmationTone;

use super::models::{ConfirmationPlan, MaintenanceAction};

pub(crate) fn maintenance_confirmation_plan(action: MaintenanceAction) -> ConfirmationPlan {
    match action {
        MaintenanceAction::CleanupExpired => ConfirmationPlan {
            title: "批量处理过期图片".to_string(),
            summary: "系统会永久删除所有已过期图片，并同步移除对应文件与数据库记录。".to_string(),
            consequences: vec![
                "只影响已经到期的图片。".to_string(),
                "处理完成后图片将无法恢复。".to_string(),
                "如果过期策略配置不正确，可能一次影响较多图片。".to_string(),
            ],
            confirm_label: "确认永久删除".to_string(),
            cancel_label: "取消".to_string(),
            tone: ConfirmationTone::Danger,
            confirm_phrase: Some("DELETE".to_string()),
            confirm_hint: Some("请输入 DELETE 以确认永久删除过期图片".to_string()),
        },
        MaintenanceAction::DeleteBackup(filename) => ConfirmationPlan {
            title: "删除备份文件".to_string(),
            summary: format!("你正在删除备份文件 {}。", filename),
            consequences: vec![
                "只会删除备份目录中的快照文件，不会直接修改当前在线数据库。".to_string(),
                "删除后这份快照将不能再下载，也不能再作为回滚依据。".to_string(),
                "建议仅删除已确认不再需要的旧备份。".to_string(),
            ],
            confirm_label: "确认删除备份".to_string(),
            cancel_label: "取消".to_string(),
            tone: ConfirmationTone::Danger,
            confirm_phrase: Some(filename.clone()),
            confirm_hint: Some(format!("请输入 {} 以确认删除", filename)),
        },
        MaintenanceAction::RestoreBackup(_) => unreachable!("restore confirmation uses precheck"),
    }
}

pub(crate) fn merge_messages(primary: &str, secondary: &str) -> String {
    match (primary.trim(), secondary.trim()) {
        ("", "") => String::new(),
        ("", secondary) => secondary.to_string(),
        (primary, "") => primary.to_string(),
        (primary, secondary) => format!("{}；{}", primary, secondary),
    }
}
