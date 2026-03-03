use crate::cache::{Cache, ImageCache};
use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::models::*;
use axum::{
    extract::{Path, State},
    Json,
};
use tracing::info;
use uuid::Uuid;

pub async fn get_categories(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<Vec<Category>>, AppError> {
    let cache_key = ImageCache::categories(auth_user.id);

    // 尝试从缓存获取
    let mut redis = state.redis.clone();
    if let Ok(Some(cached)) = Cache::get::<Vec<Category>, _>(&mut redis, &cache_key).await {
        return Ok(Json(cached));
    }

    let categories = sqlx::query_as(
        "SELECT * FROM categories WHERE user_id = $1"
    )
    .bind(auth_user.id)
    .fetch_all(&state.pool)
    .await?;

    // 缓存结果
    let ttl = state.config.cache.categories_ttl;
    let _ = Cache::set(&mut redis, &cache_key, &categories, ttl).await;

    Ok(Json(categories))
}

pub async fn create_category(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(req): Json<CreateCategoryRequest>,
) -> Result<Json<Category>, AppError> {
    if req.name.is_empty() || req.name.len() > 50 {
        return Err(AppError::EmptyCategoryName);
    }

    let category_id = Uuid::new_v4();
    let category = Category {
        id: category_id,
        user_id: auth_user.id,
        name: req.name,
        created_at: chrono::Utc::now(),
    };

    sqlx::query(
        "INSERT INTO categories (id, user_id, name, created_at) VALUES ($1, $2, $3, $4)"
    )
    .bind(category.id)
    .bind(category.user_id)
    .bind(&category.name)
    .bind(category.created_at)
    .execute(&state.pool)
    .await?;

    // 清除用户相关缓存
    let _ = state.invalidate_user_category_cache(auth_user.id).await;

    info!("Category created: {} by {}", category_id, auth_user.username);
    Ok(Json(category))
}

pub async fn delete_category(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    sqlx::query(
        "DELETE FROM categories WHERE id = $1 AND user_id = $2"
    )
    .bind(id)
    .bind(auth_user.id)
    .execute(&state.pool)
    .await?;

    // 清除用户相关缓存
    let _ = state.invalidate_user_caches(auth_user.id).await;

    info!("Category deleted: {} by {}", id, auth_user.username);
    Ok(())
}
