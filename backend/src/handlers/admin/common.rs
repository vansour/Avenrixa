use crate::db::AppState;
use crate::domain::admin::AdminDomainService;
use crate::error::AppError;
use std::sync::Arc;

pub(super) fn admin_service(state: &AppState) -> Result<Arc<AdminDomainService>, AppError> {
    Ok(state.admin_domain_service.clone())
}
