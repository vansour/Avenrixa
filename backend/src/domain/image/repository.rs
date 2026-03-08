//! 图片数据访问 trait
//!
//! 定义图片相关的数据访问接口

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Image, Category};

/// 图片数据访问 trait
#[async_trait]
#[allow(dead_code)] // trait 方法定义供将来扩展使用
pub trait ImageRepository: Send + Sync {
    /// 根据ID查找图片
    async fn find_image_by_id(&self, id: Uuid) -> Result<Option<Image>, sqlx::Error>;

    /// 根据用户ID查找图片列表
    #[allow(clippy::too_many_arguments)]
    async fn find_images_by_user_paginated(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
        sort_by: &str,
        sort_order: &str,
        search: Option<&str>,
        category_id: Option<Uuid>,
        tag: Option<&str>,
    ) -> Result<Vec<Image>, sqlx::Error>;

    /// 统计用户图片数量
    async fn count_images_by_user(&self, user_id: Uuid) -> Result<i64, sqlx::Error>;

    /// 创建图片
    async fn create_image(&self, image: &Image) -> Result<(), sqlx::Error>;

    /// 更新图片
    async fn update_image(&self, image: &Image) -> Result<(), sqlx::Error>;

    /// 软删除图片
    async fn soft_delete_image(&self, id: Uuid) -> Result<(), sqlx::Error>;

    /// 永久删除图片
    async fn hard_delete_image(&self, id: Uuid) -> Result<(), sqlx::Error>;

    /// 根据哈希查找图片（用于去重）
    async fn find_image_by_hash(&self, hash: &str, user_id: Uuid) -> Result<Option<Image>, sqlx::Error>;

    /// 全局根据哈希查找图片
    async fn find_image_by_hash_global(&self, hash: &str) -> Result<Option<Image>, sqlx::Error>;

    /// 查找已删除的图片
    async fn find_deleted_images_by_user(&self, user_id: Uuid) -> Result<Vec<Image>, sqlx::Error>;

    /// Cursor-based 分页查找图片
    async fn find_images_by_user_cursor(
        &self,
        user_id: Uuid,
        cursor: Option<(chrono::DateTime<chrono::Utc>, Uuid)>,
        limit: i32,
    ) -> Result<Vec<Image>, sqlx::Error>;
}

/// 分类数据访问 trait
#[async_trait]
#[allow(dead_code)] // trait 方法定义供将来扩展使用
pub trait CategoryRepository: Send + Sync {
    /// 查找用户所有分类
    async fn find_categories_by_user(&self, user_id: Uuid) -> Result<Vec<Category>, sqlx::Error>;

    /// 根据ID查找分类
    async fn find_category_by_id(&self, id: Uuid) -> Result<Option<Category>, sqlx::Error>;

    /// 创建分类
    async fn create_category(&self, category: &Category) -> Result<(), sqlx::Error>;

    /// 删除分类
    async fn delete_category(&self, id: Uuid) -> Result<(), sqlx::Error>;
}

/// PostgreSQL 图片仓库实现
pub struct PostgresImageRepository {
    pool: PgPool,
}

impl PostgresImageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ImageRepository for PostgresImageRepository {
    async fn find_image_by_id(&self, id: Uuid) -> Result<Option<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(
            "SELECT id, user_id, category_id, filename, thumbnail, original_filename, size, hash, format, views, status, expires_at, deleted_at, created_at
             FROM images WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn find_images_by_user_paginated(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
        sort_by: &str,
        sort_order: &str,
        search: Option<&str>,
        category_id: Option<Uuid>,
        tag: Option<&str>,
    ) -> Result<Vec<Image>, sqlx::Error> {
        let order_clause = format!("{} {}", sort_by, sort_order);

        let images = match (search, category_id, tag) {
            (Some(s), Some(cid), Some(t)) if !s.is_empty() => {
                let sql = format!(
                    "SELECT *, COUNT(*) OVER() as total_count FROM images WHERE user_id = $1 AND deleted_at IS NULL AND status = 'active' AND category_id = $2 AND $3 IN (SELECT tag FROM image_tags WHERE image_id = images.id) AND (filename ILIKE $4 OR id::text IN (SELECT tag FROM image_tags WHERE tag ILIKE $5 AND image_id = images.id)) ORDER BY {} LIMIT $6 OFFSET $7",
                    order_clause
                );
                sqlx::query_as::<_, Image>(&sql)
                    .bind(user_id)
                    .bind(cid)
                    .bind(t)
                    .bind(format!("%{}%", s))
                    .bind(format!("%{}%", s))
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            (Some(s), Some(cid), None) if !s.is_empty() => {
                let sql = format!(
                    "SELECT *, COUNT(*) OVER() as total_count FROM images WHERE user_id = $1 AND deleted_at IS NULL AND status = 'active' AND category_id = $2 AND (filename ILIKE $3 OR id::text IN (SELECT tag FROM image_tags WHERE tag ILIKE $4 AND image_id = images.id)) ORDER BY {} LIMIT $5 OFFSET $6",
                    order_clause
                );
                sqlx::query_as::<_, Image>(&sql)
                    .bind(user_id)
                    .bind(cid)
                    .bind(format!("%{}%", s))
                    .bind(format!("%{}%", s))
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            (Some(s), None, Some(t)) if !s.is_empty() => {
                let sql = format!(
                    "SELECT *, COUNT(*) OVER() as total_count FROM images WHERE user_id = $1 AND deleted_at IS NULL AND status = 'active' AND $2 IN (SELECT tag FROM image_tags WHERE image_id = images.id) AND (filename ILIKE $3 OR id::text IN (SELECT tag FROM image_tags WHERE tag ILIKE $4 AND image_id = images.id)) ORDER BY {} LIMIT $5 OFFSET $6",
                    order_clause
                );
                sqlx::query_as::<_, Image>(&sql)
                    .bind(user_id)
                    .bind(t)
                    .bind(format!("%{}%", s))
                    .bind(format!("%{}%", s))
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            (Some(s), None, None) if !s.is_empty() => {
                let sql = format!(
                    "SELECT *, COUNT(*) OVER() as total_count FROM images WHERE user_id = $1 AND deleted_at IS NULL AND status = 'active' AND (filename ILIKE $2 OR id::text IN (SELECT tag FROM image_tags WHERE tag ILIKE $3 AND image_id = images.id)) ORDER BY {} LIMIT $4 OFFSET $5",
                    order_clause
                );
                sqlx::query_as::<_, Image>(&sql)
                    .bind(user_id)
                    .bind(format!("%{}%", s))
                    .bind(format!("%{}%", s))
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            (None, Some(cid), Some(t)) => {
                let sql = format!(
                    "SELECT *, COUNT(*) OVER() as total_count FROM images WHERE user_id = $1 AND deleted_at IS NULL AND status = 'active' AND category_id = $2 AND $3 IN (SELECT tag FROM image_tags WHERE image_id = images.id) ORDER BY {} LIMIT $4 OFFSET $5",
                    order_clause
                );
                sqlx::query_as::<_, Image>(&sql)
                    .bind(user_id)
                    .bind(cid)
                    .bind(t)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            (None, Some(cid), None) => {
                let sql = format!(
                    "SELECT *, COUNT(*) OVER() as total_count FROM images WHERE user_id = $1 AND deleted_at IS NULL AND status = 'active' AND category_id = $2 ORDER BY {} LIMIT $3 OFFSET $4",
                    order_clause
                );
                sqlx::query_as::<_, Image>(&sql)
                    .bind(user_id)
                    .bind(cid)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            (None, None, Some(t)) => {
                let sql = format!(
                    "SELECT *, COUNT(*) OVER() as total_count FROM images WHERE user_id = $1 AND deleted_at IS NULL AND status = 'active' AND $2 IN (SELECT tag FROM image_tags WHERE image_id = images.id) ORDER BY {} LIMIT $3 OFFSET $4",
                    order_clause
                );
                sqlx::query_as::<_, Image>(&sql)
                    .bind(user_id)
                    .bind(t)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            _ => {
                let sql = format!(
                    "SELECT *, COUNT(*) OVER() as total_count FROM images WHERE user_id = $1 AND deleted_at IS NULL AND status = 'active' ORDER BY {} LIMIT $2 OFFSET $3",
                    order_clause
                );
                sqlx::query_as::<_, Image>(&sql)
                    .bind(user_id)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await?
            }
        };

        Ok(images)
    }

    async fn count_images_by_user(&self, user_id: Uuid) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) as count FROM images WHERE user_id = $1 AND deleted_at IS NULL"
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0)
    }

    async fn create_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO images (id, user_id, category_id, filename, thumbnail, original_filename, size, hash, format, views, status, expires_at, deleted_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"
        )
        .bind(image.id)
        .bind(image.user_id)
        .bind(image.category_id)
        .bind(&image.filename)
        .bind(&image.thumbnail)
        .bind(&image.original_filename)
        .bind(image.size)
        .bind(&image.hash)
        .bind(&image.format)
        .bind(image.views)
        .bind(&image.status)
        .bind(image.expires_at)
        .bind(image.deleted_at)
        .bind(image.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE images SET filename = $1, thumbnail = $2, category_id = $3, status = $4, expires_at = $5
             WHERE id = $6"
        )
        .bind(&image.filename)
        .bind(&image.thumbnail)
        .bind(image.category_id)
        .bind(&image.status)
        .bind(image.expires_at)
        .bind(image.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn soft_delete_image(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE images SET deleted_at = NOW() WHERE id = $1"
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn hard_delete_image(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM images WHERE id = $1")
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_image_by_hash(&self, hash: &str, user_id: Uuid) -> Result<Option<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(
            "SELECT id, user_id, category_id, filename, thumbnail, original_filename, size, hash, format, views, status, expires_at, deleted_at, created_at
             FROM images WHERE hash = $1 AND user_id = $2 AND deleted_at IS NULL"
        )
        .bind(hash)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn find_image_by_hash_global(&self, hash: &str) -> Result<Option<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(
            "SELECT id, user_id, category_id, filename, thumbnail, original_filename, size, hash, format, views, status, expires_at, deleted_at, created_at
             FROM images WHERE hash = $1 AND deleted_at IS NULL LIMIT 1"
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
    }

    async fn find_deleted_images_by_user(&self, user_id: Uuid) -> Result<Vec<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(
            "SELECT * FROM images WHERE user_id = $1 AND deleted_at IS NOT NULL ORDER BY deleted_at DESC"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn find_images_by_user_cursor(
        &self,
        user_id: Uuid,
        cursor: Option<(chrono::DateTime<chrono::Utc>, Uuid)>,
        limit: i32,
    ) -> Result<Vec<Image>, sqlx::Error> {
        match cursor {
            Some((created_at, id)) => {
                sqlx::query_as::<_, Image>(
                    "SELECT * FROM images
                     WHERE user_id = $1 AND deleted_at IS NULL AND (created_at, id) < ($2, $3)
                     ORDER BY created_at DESC, id DESC
                     LIMIT $4"
                )
                .bind(user_id)
                .bind(created_at)
                .bind(id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, Image>(
                    "SELECT * FROM images
                     WHERE user_id = $1 AND deleted_at IS NULL
                     ORDER BY created_at DESC, id DESC
                     LIMIT $2"
                )
                .bind(user_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
            }
        }
    }
}

/// PostgreSQL 分类仓库实现
pub struct PostgresCategoryRepository {
    pool: PgPool,
}

impl PostgresCategoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CategoryRepository for PostgresCategoryRepository {
    async fn find_categories_by_user(&self, user_id: Uuid) -> Result<Vec<Category>, sqlx::Error> {
        sqlx::query_as::<_, Category>(
            "SELECT id, user_id, name, created_at FROM categories WHERE user_id = $1 ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn find_category_by_id(&self, id: Uuid) -> Result<Option<Category>, sqlx::Error> {
        sqlx::query_as::<_, Category>(
            "SELECT id, user_id, name, created_at FROM categories WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn create_category(&self, category: &Category) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO categories (id, user_id, name, created_at) VALUES ($1, $2, $3, $4)"
        )
        .bind(category.id)
        .bind(category.user_id)
        .bind(&category.name)
        .bind(category.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_category(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM categories WHERE id = $1")
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
