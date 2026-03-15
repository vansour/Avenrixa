use crate::components::ConfirmationModal;
use dioxus::prelude::*;

use super::super::{MaintenanceAction, PendingMaintenanceAction};

#[component]
pub(super) fn PendingMaintenanceActionModal(
    pending: PendingMaintenanceAction,
    is_submitting: bool,
    #[props(default)] on_close: EventHandler<()>,
    #[props(default)] on_confirm_action: EventHandler<MaintenanceAction>,
) -> Element {
    let action = pending.action.clone();

    rsx! {
        ConfirmationModal {
            title: pending.plan.title.clone(),
            summary: pending.plan.summary.clone(),
            consequences: pending.plan.consequences.clone(),
            confirm_label: pending.plan.confirm_label.clone(),
            cancel_label: pending.plan.cancel_label.clone(),
            tone: pending.plan.tone,
            confirm_phrase: pending.plan.confirm_phrase.clone(),
            confirm_hint: pending.plan.confirm_hint.clone(),
            is_submitting,
            on_close: move || on_close.call(()),
            on_confirm: move || on_confirm_action.call(action.clone()),
        }
    }
}
