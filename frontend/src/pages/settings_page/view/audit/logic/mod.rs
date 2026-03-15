mod meta;
mod summary;
mod title;

use crate::types::api::AuditLog;

pub(super) fn audit_actor_label(log: &AuditLog) -> String {
    meta::audit_actor_label(log)
}

pub(super) fn audit_category_class(log: &AuditLog) -> &'static str {
    meta::audit_category_class(log)
}

pub(super) fn audit_category_label(log: &AuditLog) -> &'static str {
    meta::audit_category_label(log)
}

pub(super) fn audit_risk_label(log: &AuditLog) -> &'static str {
    meta::audit_risk_label(log)
}

pub(super) fn audit_summary(log: &AuditLog) -> String {
    summary::audit_summary(log)
}

pub(super) fn audit_target_type_label(target_type: &str) -> String {
    title::audit_target_type_label(target_type)
}

pub(super) fn audit_title(log: &AuditLog) -> String {
    title::audit_title(log)
}
