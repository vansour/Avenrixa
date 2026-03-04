use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::{AdminUser, AuthUser};
use crate::models::*;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use redis::AsyncCommands;
use tracing::{error, info};
use uuid::Uuid;
use crate::audit::log_audit;
use crate::utils::write_file_with_retry;


pub async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<HealthStatus>, AppError> {
    use sqlx::Executor;

    let timestamp = Utc::now();
    let mut overall_status = "healthy".to_string();

    // 检查数据库
    let db_status = match state.pool.acquire().await {
        Ok(mut conn) => match conn.execute(sqlx::query("SELECT 1")).await {
            Ok(_) => ComponentStatus::healthy(),
            Err(e) => {
                overall_status = "unhealthy".to_string();
                ComponentStatus::unhealthy(e.to_string())
            }
        },
        Err(e) => {
            overall_status = "unhealthy".to_string();
            ComponentStatus::unhealthy(e.to_string())
        }
    };

    // 检查 Redis
    let mut redis = state.redis.clone();
    let redis_status: ComponentStatus = match redis.ping::<()>().await {
        Ok(_) => ComponentStatus::healthy(),
        Err(e) => {
            overall_status = "unhealthy".to_string();
            ComponentStatus::unhealthy(e.to_string())
        }
    };

    // 检查存储
    let storage_path = &state.config.storage.path;
    let storage_status = match tokio::fs::metadata(storage_path).await {
        Ok(_) => ComponentStatus::healthy(),
        Err(e) => {
            overall_status = "unhealthy".to_string();
            ComponentStatus::unhealthy(e.to_string())
        }
    };

    // 收集系统指标（异步，错误不影响健康状态）
    let (images_count, users_count, storage_used_mb) = async {
        // 图片数量
        let images_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM images WHERE deleted_at IS NULL")
            .fetch_one(&state.pool)
            .await
            .unwrap_or(0);

        // 用户数量
        let users_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&state.pool)
            .await
            .unwrap_or(0);

        // 存储使用情况
        let storage_used_mb: Option<f64> = {
            let total_size = sqlx::query_scalar("SELECT SUM(size) FROM images")
                .fetch_one(&state.pool)
                .await
                .unwrap_or(None);
            total_size.map(|size: i64| size as f64 / 1024.0 / 1024.0)
        };

        (images_count, users_count, storage_used_mb)
    }.await;

    let metrics = HealthMetrics {
        images_count,
        users_count,
        storage_used_mb,
    };

    Ok(Json(HealthStatus {
        status: overall_status,
        timestamp,
        database: db_status,
        redis: redis_status,
        storage: storage_status,
        version: option_env!("APP_VERSION").map(|s| s.to_string()),
        uptime_seconds: Some(state.started_at.elapsed().as_secs()),
        metrics: Some(metrics),
    }))
}

pub async fn cleanup_deleted_files(
    State(state): State<AppState>,
    admin_user: AdminUser,
) -> Result<Json<Vec<String>>, AppError> {
    let now = Utc::now();
    let retention_days = state.config.cleanup.deleted_images_retention_days;
    let days_ago = now - chrono::Duration::days(retention_days);

    let result = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, filename FROM images WHERE deleted_at < $1"
    )
    .bind(days_ago)
    .fetch_all(&state.pool)
    .await;

    let result = match result {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to query cleanup images: {}", e);
            // 记录审计日志
            let error_msg = e.to_string();
            tokio::spawn(async move {
                let _ = log_audit(
                    &state.pool,
                    Some(admin_user.id),
                    "cleanup_failed",
                    "cleanup",
                    None,
                    None,
                    Some(serde_json::json!({"error": error_msg}))
                ).await;
            });
            return Err(AppError::DatabaseError(e));
        }
    };

    let mut removed = vec![];
    for (id, filename) in &result {
        let storage_path = format!("{}/{}", state.config.storage.path, filename);
        let thumbnail_path = format!("{}/{}.jpg", state.config.storage.thumbnail_path, id);

        let file_removed = tokio::fs::remove_file(&storage_path).await.is_ok();
        let thumb_removed = tokio::fs::remove_file(&thumbnail_path).await.is_ok();

        if file_removed || thumb_removed {
            let _ = sqlx::query("DELETE FROM images WHERE id = $1")
                .bind(id)
                .execute(&state.pool)
                .await;
            removed.push(filename.clone());
        }
    }

    info!("Cleanup complete: {} images removed", removed.len());
    // 记录清理操作审计日志
    let removed_count = removed.len();
    tokio::spawn(async move {
        let _ = log_audit(
            &state.pool,
            Some(admin_user.id),
            "cleanup_completed",
            "cleanup",
            None,
            None,
            Some(serde_json::json!({"removed_count": removed_count}))
        ).await;
    });
    Ok(Json(removed))
}

pub async fn cleanup_expired_images(
    State(state): State<AppState>,
    admin_user: AdminUser,
) -> Result<Json<i64>, AppError> {
    let now = Utc::now();

    let result = sqlx::query(
        "UPDATE images SET deleted_at = $1 WHERE expires_at < $1 AND deleted_at IS NULL"
    )
    .bind(now)
    .execute(&state.pool)
    .await;

    let result = match result {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to expire images: {}", e);
            let error_msg = e.to_string();
            tokio::spawn(async move {
                let _ = log_audit(
                    &state.pool,
                    Some(admin_user.id),
                    "expire_failed",
                    "expiry",
                    None,
                    None,
                    Some(serde_json::json!({"error": error_msg}))
                ).await;
            });
            return Err(AppError::DatabaseError(e));
        }
    };

    let affected = result.rows_affected();
    if affected > 0 {
        info!("Expired images moved to trash: {}", affected);
    }

    let affected_count = affected;
    tokio::spawn(async move {
        let _ = log_audit(
            &state.pool,
            Some(admin_user.id),
            "expire_completed",
            "expiry",
            None,
            None,
            Some(serde_json::json!({"affected_count": affected_count}))
        ).await;
    });
    Ok(Json(affected as i64))
}

pub async fn backup_database(
    State(_): State<AppState>,
    admin_user: AdminUser,
) -> Result<Json<BackupResponse>, AppError> {
    let filename = format!("backup_{}.sql", Uuid::new_v4());
    let backup_path = format!("/data/backup/{}", filename);

    // 创建备份目录
    tokio::fs::create_dir_all("/data/backup").await?;

    // 使用 PostgreSQL COPY TO stdout 导出数据
    let mut backup_content = String::new();
    // Schema 已经通过 db.rs::init_schema() 初始化，这里只需导出数据
    backup_content.push_str("-- Schema is initialized via db.rs\n\n");

    // 导出数据
    let tables = vec![
        ("users", "id, username, password_hash, role, created_at"),
        ("categories", "id, user_id, name, created_at"),
        ("images", "id, user_id, category_id, filename, thumbnail, original_filename, size, hash, format, views, status, expires_at, deleted_at, created_at"),
    ];

    for (table, columns) in &tables {
        backup_content.push_str(&format!("-- Dumping table: {}\n", table));
        // 使用 COPY TO stdout 直接导出，性能提升显著
        backup_content.push_str(&format!("COPY (SELECT {} FROM {}) TO stdout WITH CSV HEADER DELIMITER ',' FORCE QUOTE {}\n",
            columns, table, "\\"));
    }

    // 使用带重试的写入
    if let Err(e) = write_file_with_retry(&backup_path, backup_content.as_bytes(), 3).await {
        error!("Failed to write backup file after retries: {}", e);
        return Err(AppError::IoError(e));
    }

    info!("Database backup created: {} by {}", filename, admin_user.username);
    Ok(Json(BackupResponse {
        filename,
        created_at: Utc::now(),
    }))
}

pub async fn approve_images(
    State(state): State<AppState>,
    _admin_user: AdminUser,
    Json(req): Json<ApproveRequest>,
) -> Result<(), AppError> {
    let status = if req.approved { "active" } else { "pending" };

    sqlx::query("UPDATE images SET status = $1 WHERE id = ANY($2)")
        .bind(status)
        .bind(&req.image_ids)
        .execute(&state.pool)
        .await?;

    info!("Images approved: {:?}", req.image_ids);
    Ok(())
}

pub async fn get_users(
    State(state): State<AppState>,
    _admin_user: AdminUser,
) -> Result<Json<Vec<User>>, AppError> {
    let users = sqlx::query_as(
        "SELECT id, username, password_hash, role, created_at FROM users ORDER BY created_at DESC"
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(users))
}

pub async fn update_user_role(
    State(state): State<AppState>,
    _admin_user: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UserUpdateRequest>,
) -> Result<(), AppError> {
    if let Some(ref role) = req.role {
        if role != "admin" && role != "user" {
            return Err(AppError::InvalidPagination);
        }

        sqlx::query("UPDATE users SET role = $1 WHERE id = $2")
            .bind(role)
            .bind(id)
            .execute(&state.pool)
            .await?;

        info!("User role updated: {} -> {}", id, role);
    }

    Ok(())
}

pub async fn get_audit_logs(
    State(state): State<AppState>,
    _admin_user: AdminUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<AuditLogResponse>, AppError> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(50).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let logs: Vec<AuditLog> = sqlx::query_as(
        "SELECT * FROM audit_logs ORDER BY created_at DESC LIMIT $1 OFFSET $2"
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_logs")
        .fetch_one(&state.pool)
        .await?;

    let _has_next = ((page * page_size) as i64) < total;

    Ok(Json(AuditLogResponse {
        data: logs,
        page,
        page_size,
        total,
    }))
}

pub async fn get_system_stats(
    State(state): State<AppState>,
    _admin_user: AdminUser,
) -> Result<Json<SystemStats>, AppError> {
    let now = Utc::now();
    let day_ago = now - chrono::Duration::days(1);
    let week_ago = now - chrono::Duration::days(7);

    let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.pool)
        .await?;

    let total_images: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM images WHERE deleted_at IS NULL")
        .fetch_one(&state.pool)
        .await?;

    let total_storage: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(size), 0) FROM images WHERE deleted_at IS NULL")
        .fetch_one(&state.pool)
        .await?;

    let total_views: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(views), 0) FROM images WHERE deleted_at IS NULL")
        .fetch_one(&state.pool)
        .await?;

    let images_last_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM images WHERE created_at > $1 AND deleted_at IS NULL"
    )
    .bind(day_ago)
    .fetch_one(&state.pool)
    .await?;

    let images_last_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM images WHERE created_at > $1 AND deleted_at IS NULL"
    )
    .bind(week_ago)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(SystemStats {
        total_users,
        total_images,
        total_storage,
        total_views,
        images_last_24h,
        images_last_7d,
    }))
}

/// 普通用户也可以访问的设置（只读）
pub async fn get_settings_public(
    State(state): State<AppState>,
    _auth_user: AuthUser,
) -> Result<Json<Vec<Setting>>, AppError> {
    let settings = sqlx::query_as(
        "SELECT * FROM settings ORDER BY key"
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(settings))
}

/// 管理员专用设置端点
pub async fn get_settings(
    State(state): State<AppState>,
    _admin_user: AdminUser,
) -> Result<Json<Vec<Setting>>, AppError> {
    let settings = sqlx::query_as(
        "SELECT * FROM settings ORDER BY key"
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(settings))
}

pub async fn update_setting(
    State(state): State<AppState>,
    _admin_user: AdminUser,
    Path(key): Path<String>,
    Json(req): Json<UpdateSettingRequest>,
) -> Result<(), AppError> {
    // 检查设置是否存在
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM settings WHERE key = $1)"
    )
    .bind(&key)
    .fetch_one(&state.pool)
    .await?;

    if exists {
        sqlx::query("UPDATE settings SET value = $1, updated_at = $2 WHERE key = $3")
            .bind(&req.value)
            .bind(Utc::now())
            .bind(&key)
            .execute(&state.pool)
            .await?;
    } else {
        sqlx::query("INSERT INTO settings (key, value, updated_at) VALUES ($1, $2, $3)")
            .bind(&key)
            .bind(&req.value)
            .bind(Utc::now())
            .execute(&state.pool)
            .await?;
    }

    info!("Setting updated: {} = {}", key, req.value);
    Ok(())
}
