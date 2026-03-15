mod activity;
mod maintenance;
mod settings;

use crate::types::api::AuditLog;

use super::super::super::shared::format_json_details;

pub(super) fn audit_summary(log: &AuditLog) -> String {
    maintenance::audit_maintenance_summary(log)
        .or_else(|| settings::audit_settings_summary(log))
        .or_else(|| activity::audit_activity_summary(log))
        .unwrap_or_else(|| {
            log.details
                .as_ref()
                .map(format_json_details)
                .unwrap_or_else(|| "无附加详情".to_string())
        })
}
