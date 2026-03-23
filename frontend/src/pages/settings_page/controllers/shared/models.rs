use crate::components::ConfirmationTone;
use crate::types::api::UserRole;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ConfirmationPlan {
    pub(crate) title: String,
    pub(crate) summary: String,
    pub(crate) consequences: Vec<String>,
    pub(crate) confirm_label: String,
    pub(crate) cancel_label: String,
    pub(crate) tone: ConfirmationTone,
    pub(crate) confirm_phrase: Option<String>,
    pub(crate) confirm_hint: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum MaintenanceAction {
    CleanupExpired,
    DeleteBackup(String),
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PendingMaintenanceAction {
    pub(crate) action: MaintenanceAction,
    pub(crate) plan: ConfirmationPlan,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PendingUserRoleChange {
    pub(crate) user_id: String,
    pub(crate) email: String,
    pub(crate) next_role: UserRole,
    pub(crate) plan: ConfirmationPlan,
}
