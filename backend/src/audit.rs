use crate::db::DatabasePool;
use crate::models::*;
use tracing::info;
use uuid::Uuid;

pub async fn log_audit_db(
    database: &DatabasePool,
    user_id: Option<Uuid>,
    action: &str,
    target_type: &str,
    target_id: Option<Uuid>,
    ip_address: Option<&str>,
    details: Option<serde_json::Value>,
) {
    match database {
        DatabasePool::Postgres(pool) => {
            insert_postgres_audit(
                pool,
                user_id,
                action,
                target_type,
                target_id,
                ip_address,
                details,
            )
            .await;
        }
    }
}

async fn insert_postgres_audit(
    pool: &sqlx::PgPool,
    user_id: Option<Uuid>,
    action: &str,
    target_type: &str,
    target_id: Option<Uuid>,
    ip_address: Option<&str>,
    details: Option<serde_json::Value>,
) {
    let audit_log = AuditLogRecord {
        id: Uuid::new_v4(),
        user_id,
        action: action.to_string(),
        target_type: target_type.to_string(),
        target_id,
        details,
        ip_address: ip_address.map(|s| s.to_string()),
        created_at: chrono::Utc::now(),
    };

    let result = sqlx::query(
        "INSERT INTO audit_logs (id, user_id, action, target_type, target_id, details, ip_address, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
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
    .await;

    match result {
        Ok(_) => info!(
            action = %action,
            target_type = %target_type,
            target_id = ?target_id,
            user_id = ?user_id,
            "Audit log recorded"
        ),
        Err(e) => tracing::error!("Failed to record PostgreSQL audit log: {}", e),
    }
}
