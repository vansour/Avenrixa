//! Cursor-based 图片分页查询模块
//! 支持 cursor-based 分页以提高性能
//! 使用完全参数化查询，避免 SQL 注入风险

use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::models::{Image, CursorPaginated};
use axum::extract::{Query, State};
use axum::Json;
use chrono::Utc;
use tracing::info;
use uuid::Uuid;
use sqlx::PgPool;

/// Cursor-based 图片分页
pub async fn get_images(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(params): Query<crate::models::PaginationParams>,
) -> Result<Json<CursorPaginated<Image>>, AppError> {
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    // 验证排序字段
    let sort_by = match params.sort_by.as_deref() {
        Some(s) if is_valid_sort_field(s) => s.to_string(),
        _ => "created_at".to_string(),
    };

    let sort_order = match params.sort_order.as_deref() {
        Some(s) if is_valid_sort_order(s) => s.to_uppercase(),
        _ => "DESC".to_string(),
    };

    let search = params.search.as_deref();
    let category_id = params.category_id;
    let tag = params.tag.as_deref();

    // 解码 cursor
    let (cursor_created_at, cursor_id) = match &params.cursor {
        Some((created_at, id)) => {
            info!("Using cursor: created_at={}, id={}", created_at, id);
            (Some(*created_at), Some(id.clone()))
        }
        _ => (None, None),
    };

    // 根据查询条件选择合适的预编译 SQL
    let images = match (search, category_id, tag, cursor_created_at) {
        // 有搜索 + 有 cursor
        (Some(_), _, _, Some(_)) => {
            execute_search_cursor_query(
                &state.pool,
                auth_user.id,
                format!("%{}%", search.unwrap()),
                cursor_created_at.unwrap(),
                cursor_id.as_deref().unwrap_or_default().to_string(),
                page_size,
            ).await?
        }

        // 有搜索 + 无 cursor
        (Some(_), _, _, None) => {
            execute_search_query(
                &state.pool,
                auth_user.id,
                format!("%{}%", search.unwrap()),
                page_size,
            ).await?
        }

        // 无搜索 + 有分类 + 有 cursor
        (None, Some(_), _, Some(_)) => {
            execute_category_cursor_query(
                &state.pool,
                auth_user.id,
                category_id.unwrap(),
                cursor_created_at.unwrap(),
                cursor_id.as_deref().unwrap_or_default().to_string(),
                page_size,
            ).await?
        }

        // 无搜索 + 有分类 + 无 cursor
        (None, Some(_), _, None) => {
            execute_category_query(
                &state.pool,
                auth_user.id,
                category_id.unwrap(),
                page_size,
            ).await?
        }

        // 无搜索 + 有标签 + 有 cursor
        (None, None, Some(_), Some(_)) => {
            execute_tag_cursor_query(
                &state.pool,
                auth_user.id,
                tag.unwrap().to_string(),
                cursor_created_at.unwrap(),
                cursor_id.as_deref().unwrap_or_default().to_string(),
                page_size,
            ).await?
        }

        // 无搜索 + 有标签 + 无 cursor
        (None, None, Some(_), None) => {
            execute_tag_query(
                &state.pool,
                auth_user.id,
                tag.unwrap().to_string(),
                page_size,
            ).await?
        }

        // 有 cursor（任何排序字段）
        (None, None, None, Some(_)) => {
            execute_cursor_query(
                &state.pool,
                get_cursor_query(&sort_by, &sort_order),
                auth_user.id,
                cursor_created_at.unwrap(),
                cursor_id.as_deref().unwrap_or_default().to_string(),
                page_size,
            ).await?
        }

        // 无 cursor（默认情况）
        (None, None, None, None) => {
            execute_default_query(
                &state.pool,
                get_default_query(&sort_by, &sort_order),
                auth_user.id,
                page_size,
            ).await?
        }
    };

    let next_cursor = images
        .last()
        .map(|img| (img.created_at, img.id.to_string()));

    Ok(Json(CursorPaginated {
        data: images,
        next_cursor,
    }))
}

// ==================== 预编译 SQL 查询 ====================

// 基础查询（无筛选，无 cursor）
const QUERY_BASE_DESC: &str = r#"
    SELECT * FROM images
    WHERE user_id = $1
      AND deleted_at IS NULL
      AND status = 'active'
    ORDER BY created_at DESC
    LIMIT $2
"#;

const QUERY_BASE_ASC: &str = r#"
    SELECT * FROM images
    WHERE user_id = $1
      AND deleted_at IS NULL
      AND status = 'active'
    ORDER BY created_at ASC
    LIMIT $2
"#;

// 带搜索的查询（无 cursor）
const QUERY_SEARCH: &str = r#"
    SELECT * FROM images
    WHERE user_id = $1
      AND deleted_at IS NULL
      AND status = 'active'
      AND (
          filename ILIKE $2
          OR id IN (SELECT image_id FROM image_tags WHERE tag ILIKE $2)
      )
    ORDER BY created_at DESC
    LIMIT $3
"#;

// 带分类的查询（无 cursor）
const QUERY_CATEGORY: &str = r#"
    SELECT * FROM images
    WHERE user_id = $1
      AND deleted_at IS NULL
      AND status = 'active'
      AND category_id = $2
    ORDER BY created_at DESC
    LIMIT $3
"#;

// 带标签的查询（无 cursor）
const QUERY_TAG: &str = r#"
    SELECT * FROM images
    WHERE user_id = $1
      AND deleted_at IS NULL
      AND status = 'active'
      AND EXISTS (SELECT 1 FROM image_tags WHERE image_id = images.id AND tag = $2)
    ORDER BY created_at DESC
    LIMIT $3
"#;

// 带 cursor 的查询（动态排序）
fn get_cursor_query(sort_by: &str, sort_order: &str) -> &'static str {
    // 使用完全参数化的 cursor 查询，避免 SQL 注入
    match (sort_by, sort_order) {
        ("created_at", "DESC") => QUERY_CURSOR_CREATED_DESC,
        ("created_at", "ASC") => QUERY_CURSOR_CREATED_ASC,
        ("size", "DESC") => QUERY_CURSOR_SIZE_DESC,
        ("size", "ASC") => QUERY_CURSOR_SIZE_ASC,
        ("views", "DESC") => QUERY_CURSOR_VIEWS_DESC,
        ("views", "ASC") => QUERY_CURSOR_VIEWS_ASC,
        ("filename", "DESC") => QUERY_CURSOR_FILENAME_DESC,
        ("filename", "ASC") => QUERY_CURSOR_FILENAME_ASC,
        _ => QUERY_CURSOR_CREATED_DESC, // 默认
    }
}

// 带搜索 + cursor 的查询
const QUERY_SEARCH_CURSOR: &str = r#"
    SELECT * FROM images
    WHERE user_id = $1
      AND deleted_at IS NULL
      AND status = 'active'
      AND (
          filename ILIKE $2
          OR id IN (SELECT image_id FROM image_tags WHERE tag ILIKE $2)
      )
      AND (
          created_at < $3
          OR (created_at = $3 AND id < $4)
      )
    ORDER BY created_at DESC
    LIMIT $5
"#;

// 带分类 + cursor 的查询
const QUERY_CATEGORY_CURSOR: &str = r#"
    SELECT * FROM images
    WHERE user_id = $1
      AND deleted_at IS NULL
      AND status = 'active'
      AND category_id = $2
      AND (
          created_at < $3
          OR (created_at = $3 AND id < $4)
      )
    ORDER BY created_at DESC
    LIMIT $5
"#;

// 带标签 + cursor 的查询
const QUERY_TAG_CURSOR: &str = r#"
    SELECT * FROM images
    WHERE user_id = $1
      AND deleted_at IS NULL
      AND status = 'active'
      AND EXISTS (SELECT 1 FROM image_tags WHERE image_id = images.id AND tag = $2)
      AND (
          created_at < $3
          OR (created_at = $3 AND id < $4)
      )
    ORDER BY created_at DESC
    LIMIT $5
"#;

// Cursor 查询变体
const QUERY_CURSOR_CREATED_DESC: &str = r#"
    SELECT * FROM images
    WHERE user_id = $1
      AND deleted_at IS NULL
      AND status = 'active'
      AND (
          created_at < $2
          OR (created_at = $2 AND id < $3)
      )
    ORDER BY created_at DESC
    LIMIT $4
"#;

const QUERY_CURSOR_CREATED_ASC: &str = r#"
    SELECT * FROM images
    WHERE user_id = $1
      AND deleted_at IS NULL
      AND status = 'active'
      AND (
          created_at > $2
          OR (created_at = $2 AND id > $3)
      )
    ORDER BY created_at ASC
    LIMIT $4
"#;

const QUERY_CURSOR_SIZE_DESC: &str = r#"
    SELECT * FROM images
    WHERE user_id = $1
      AND deleted_at IS NULL
      AND status = 'active'
      AND (
          size < $2
          OR (size = $2 AND id < $3)
      )
    ORDER BY size DESC, created_at DESC
    LIMIT $4
"#;

const QUERY_CURSOR_SIZE_ASC: &str = r#"
    SELECT * FROM images
    WHERE user_id = $1
      AND deleted_at IS NULL
      AND status = 'active'
      AND (
          size > $2
          OR (size = $2 AND id > $3)
      )
    ORDER BY size ASC, created_at DESC
    LIMIT $4
"#;

const QUERY_CURSOR_VIEWS_DESC: &str = r#"
    SELECT * FROM images
    WHERE user_id = $1
      AND deleted_at IS NULL
      AND status = 'active'
      AND (
          views < $2
          OR (views = $2 AND id < $3)
      )
    ORDER BY views DESC, created_at DESC
    LIMIT $4
"#;

const QUERY_CURSOR_VIEWS_ASC: &str = r#"
    SELECT * FROM images
    WHERE user_id = $1
      AND deleted_at IS NULL
      AND status = 'active'
      AND (
          views > $2
          OR (views = $2 AND id > $3)
      )
    ORDER BY views ASC, created_at DESC
    LIMIT $4
"#;

const QUERY_CURSOR_FILENAME_DESC: &str = r#"
    SELECT * FROM images
    WHERE user_id = $1
      AND deleted_at IS NULL
      AND status = 'active'
      AND (
          filename < $2
          OR (filename = $2 AND id < $3)
      )
    ORDER BY filename DESC, created_at DESC
    LIMIT $4
"#;

const QUERY_CURSOR_FILENAME_ASC: &str = r#"
    SELECT * FROM images
    WHERE user_id = $1
      AND deleted_at IS NULL
      AND status = 'active'
      AND (
          filename > $2
          OR (filename = $2 AND id > $3)
      )
    ORDER BY filename ASC, created_at DESC
    LIMIT $4
"#;

// ==================== 查询执行函数 ====================

/// 执行默认查询
async fn execute_default_query(
    pool: &PgPool,
    sql: &'static str,
    user_id: Uuid,
    page_size: i32,
) -> Result<Vec<Image>, AppError> {
    sqlx::query_as::<_, Image>(sql)
        .bind(user_id)
        .bind(page_size)
        .fetch_all(pool)
        .await
        .map_err(AppError::DatabaseError)
}

/// 执行带搜索的查询
async fn execute_search_query(
    pool: &PgPool,
    user_id: Uuid,
    search_pattern: String,
    page_size: i32,
) -> Result<Vec<Image>, AppError> {
    sqlx::query_as::<_, Image>(QUERY_SEARCH)
        .bind(user_id)
        .bind(&search_pattern)
        .bind(&search_pattern)  // ILIKE 用两次
        .bind(page_size)
        .fetch_all(pool)
        .await
        .map_err(AppError::DatabaseError)
}

/// 执行带分类的查询
async fn execute_category_query(
    pool: &PgPool,
    user_id: Uuid,
    category_id: Uuid,
    page_size: i32,
) -> Result<Vec<Image>, AppError> {
    sqlx::query_as::<_, Image>(QUERY_CATEGORY)
        .bind(user_id)
        .bind(category_id)
        .bind(page_size)
        .fetch_all(pool)
        .await
        .map_err(AppError::DatabaseError)
}

/// 执行带标签的查询
async fn execute_tag_query(
    pool: &PgPool,
    user_id: Uuid,
    tag: String,
    page_size: i32,
) -> Result<Vec<Image>, AppError> {
    sqlx::query_as::<_, Image>(QUERY_TAG)
        .bind(user_id)
        .bind(&tag)
        .bind(page_size)
        .fetch_all(pool)
        .await
        .map_err(AppError::DatabaseError)
}

/// 执行带 cursor 的查询
async fn execute_cursor_query(
    pool: &PgPool,
    sql: &'static str,
    user_id: Uuid,
    cursor_created_at: chrono::DateTime<Utc>,
    cursor_id: String,
    page_size: i32,
) -> Result<Vec<Image>, AppError> {
    sqlx::query_as::<_, Image>(sql)
        .bind(user_id)
        .bind(cursor_created_at)
        .bind(&cursor_id)
        .bind(page_size)
        .fetch_all(pool)
        .await
        .map_err(AppError::DatabaseError)
}

/// 执行带搜索和 cursor 的查询
async fn execute_search_cursor_query(
    pool: &PgPool,
    user_id: Uuid,
    search_pattern: String,
    cursor_created_at: chrono::DateTime<Utc>,
    cursor_id: String,
    page_size: i32,
) -> Result<Vec<Image>, AppError> {
    sqlx::query_as::<_, Image>(QUERY_SEARCH_CURSOR)
        .bind(user_id)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(cursor_created_at)
        .bind(cursor_id)
        .bind(page_size)
        .fetch_all(pool)
        .await
        .map_err(AppError::DatabaseError)
}

/// 执行带分类和 cursor 的查询
async fn execute_category_cursor_query(
    pool: &PgPool,
    user_id: Uuid,
    category_id: Uuid,
    cursor_created_at: chrono::DateTime<Utc>,
    cursor_id: String,
    page_size: i32,
) -> Result<Vec<Image>, AppError> {
    sqlx::query_as::<_, Image>(QUERY_CATEGORY_CURSOR)
        .bind(user_id)
        .bind(category_id)
        .bind(cursor_created_at)
        .bind(cursor_id)
        .bind(page_size)
        .fetch_all(pool)
        .await
        .map_err(AppError::DatabaseError)
}

/// 执行带标签和 cursor 的查询
async fn execute_tag_cursor_query(
    pool: &PgPool,
    user_id: Uuid,
    tag: String,
    cursor_created_at: chrono::DateTime<Utc>,
    cursor_id: String,
    page_size: i32,
) -> Result<Vec<Image>, AppError> {
    sqlx::query_as::<_, Image>(QUERY_TAG_CURSOR)
        .bind(user_id)
        .bind(&tag)
        .bind(cursor_created_at)
        .bind(cursor_id)
        .bind(page_size)
        .fetch_all(pool)
        .await
        .map_err(AppError::DatabaseError)
}

// ==================== 辅助函数 ====================

/// 获取默认查询（根据排序方向）
fn get_default_query(sort_by: &str, sort_order: &str) -> &'static str {
    // 根据排序字段和方向返回合适的查询
    match (sort_by, sort_order) {
        ("created_at", "DESC") => QUERY_BASE_DESC,
        ("created_at", "ASC") => QUERY_BASE_ASC,
        // 其他排序字段默认使用 DESC
        _ => QUERY_BASE_DESC,
    }
}

/// 检查是否为有效的排序字段
fn is_valid_sort_field(s: &str) -> bool {
    matches!(s, "created_at" | "size" | "views" | "filename" | "hash")
}

/// 检查是否为有效的排序顺序
fn is_valid_sort_order(s: &str) -> bool {
    matches!(s.to_uppercase().as_str(), "ASC" | "DESC")
}
