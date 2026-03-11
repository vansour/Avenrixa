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
        DatabasePool::MySql(pool) => {
            insert_mysql_audit(
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
        DatabasePool::Sqlite(pool) => {
            insert_sqlite_audit(
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
    let audit_log = AuditLog {
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

async fn insert_mysql_audit(
    pool: &sqlx::MySqlPool,
    user_id: Option<Uuid>,
    action: &str,
    target_type: &str,
    target_id: Option<Uuid>,
    ip_address: Option<&str>,
    details: Option<serde_json::Value>,
) {
    let audit_log = AuditLog {
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
        "INSERT INTO audit_logs (id, user_id, action, target_type, target_id, details, ip_address, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
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
        Err(e) => tracing::error!("Failed to record MySQL audit log: {}", e),
    }
}

async fn insert_sqlite_audit(
    pool: &sqlx::SqlitePool,
    user_id: Option<Uuid>,
    action: &str,
    target_type: &str,
    target_id: Option<Uuid>,
    ip_address: Option<&str>,
    details: Option<serde_json::Value>,
) {
    let serialized_details = details.and_then(|value| match serde_json::to_string(&value) {
        Ok(serialized) => Some(serialized),
        Err(error) => {
            tracing::error!("Failed to serialize SQLite audit details: {}", error);
            None
        }
    });

    let created_at = chrono::Utc::now();
    let result = sqlx::query(
        "INSERT INTO audit_logs (id, user_id, action, target_type, target_id, details, ip_address, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(serialized_details)
    .bind(ip_address)
    .bind(created_at)
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
        Err(e) => tracing::error!("Failed to record SQLite audit log: {}", e),
    }
}
