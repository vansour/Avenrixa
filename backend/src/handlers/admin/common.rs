use crate::db::AppState;
use crate::domain::admin::AdminDomainService;
use crate::error::AppError;
use std::sync::Arc;

pub(super) fn admin_service(state: &AppState) -> Result<Arc<AdminDomainService>, AppError> {
    state
        .admin_domain_service
        .clone()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Admin service not found"
        )))
}
