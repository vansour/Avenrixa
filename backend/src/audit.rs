use crate::db::DatabasePool;
use crate::models::AuditLogRecord;
use crate::observability::RuntimeObservability;
use std::sync::Arc;
use std::time::Instant;
use tracing::error;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuditEvent {
    user_id: Option<Uuid>,
    action: String,
    target_type: String,
    target_id: Option<Uuid>,
    ip_address: Option<String>,
    details: Option<serde_json::Value>,
}

impl AuditEvent {
    pub fn new(action: impl Into<String>, target_type: impl Into<String>) -> Self {
        Self {
            user_id: None,
            action: action.into(),
            target_type: target_type.into(),
            target_id: None,
            ip_address: None,
            details: None,
        }
    }

    pub fn with_user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_target_id(mut self, target_id: Uuid) -> Self {
        self.target_id = Some(target_id);
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

pub async fn record_audit_sync(
    database: &DatabasePool,
    observability: &RuntimeObservability,
    event: AuditEvent,
) {
    let started_at = Instant::now();
    if let Err(error) = insert_audit(database, &event).await {
        observability.record_audit_failure(started_at.elapsed(), error.to_string());
        log_audit_write_failure("sync", &event, &error);
    } else {
        observability.record_audit_success(started_at.elapsed());
    }
}

pub fn record_audit_best_effort(
    database: DatabasePool,
    observability: Arc<RuntimeObservability>,
    event: AuditEvent,
) {
    tokio::spawn(async move {
        let started_at = Instant::now();
        if let Err(error) = insert_audit(&database, &event).await {
            observability.record_audit_failure(started_at.elapsed(), error.to_string());
            log_audit_write_failure("background", &event, &error);
        } else {
            observability.record_audit_success(started_at.elapsed());
        }
    });
}

async fn insert_audit(database: &DatabasePool, event: &AuditEvent) -> Result<(), sqlx::Error> {
    match database {
        DatabasePool::Postgres(pool) => insert_postgres_audit(pool, event).await,
    }
}

async fn insert_postgres_audit(pool: &sqlx::PgPool, event: &AuditEvent) -> Result<(), sqlx::Error> {
    let audit_log = AuditLogRecord {
        id: Uuid::new_v4(),
        user_id: event.user_id,
        action: event.action.clone(),
        target_type: event.target_type.clone(),
        target_id: event.target_id,
        details: event.details.clone(),
        ip_address: event.ip_address.clone(),
        created_at: chrono::Utc::now(),
    };

    sqlx::query(
        "INSERT INTO audit_logs (id, user_id, action, target_type, target_id, details, ip_address, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(audit_log.id)
    .bind(audit_log.user_id)
    .bind(&audit_log.action)
    .bind(&audit_log.target_type)
    .bind(audit_log.target_id)
    .bind(&audit_log.details)
    .bind(&audit_log.ip_address)
    .bind(audit_log.created_at)
    .execute(pool)
    .await?;

    Ok(())
}

fn log_audit_write_failure(mode: &'static str, event: &AuditEvent, error: &sqlx::Error) {
    error!(
        audit_mode = mode,
        action = %event.action,
        target_type = %event.target_type,
        target_id = ?event.target_id,
        user_id = ?event.user_id,
        error = %error,
        "Audit log write failed"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_event_builder_sets_optional_fields() {
        let user_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let event = AuditEvent::new("user.login", "user")
            .with_user_id(user_id)
            .with_target_id(target_id)
            .with_details(serde_json::json!({ "result": "completed" }));

        assert_eq!(event.user_id, Some(user_id));
        assert_eq!(event.target_id, Some(target_id));
        assert_eq!(event.ip_address, None);
        assert_eq!(
            event.details,
            Some(serde_json::json!({ "result": "completed" }))
        );
    }
}
