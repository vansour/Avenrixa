use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

use super::PostgresImageRepository;
use super::sql::{IMAGE_SELECT_COLUMNS, IMAGE_SELECT_WITH_TOTAL_COUNT};
use crate::models::Image;

impl PostgresImageRepository {
    pub(super) async fn find_image_by_id_impl(
        &self,
        id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(&format!(
            "SELECT {} FROM images WHERE id = $1",
            IMAGE_SELECT_COLUMNS
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub(super) async fn find_images_by_user_paginated_impl(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
        category_id: Option<Uuid>,
        tag: Option<&str>,
    ) -> Result<Vec<Image>, sqlx::Error> {
        let tag = tag
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_lowercase());

        let mut builder = QueryBuilder::<Postgres>::new("SELECT ");
        builder.push(IMAGE_SELECT_WITH_TOTAL_COUNT);
        builder.push(" FROM images WHERE images.user_id = ");
        builder.push_bind(user_id);
        builder.push(" AND images.deleted_at IS NULL AND images.status = 'active'");

        if let Some(cid) = category_id {
            builder.push(" AND images.category_id = ");
            builder.push_bind(cid);
        }

        if let Some(tag_value) = tag.as_deref() {
            builder.push(
                " AND EXISTS (SELECT 1 FROM image_tags it WHERE it.image_id = images.id AND it.tag = ",
            );
            builder.push_bind(tag_value);
            builder.push(")");
        }

        builder.push(" ORDER BY images.created_at DESC, images.id DESC");
        builder.push(" LIMIT ");
        builder.push_bind(limit);
        builder.push(" OFFSET ");
        builder.push_bind(offset);

        builder
            .build_query_as::<Image>()
            .fetch_all(&self.pool)
            .await
    }

    pub(super) async fn count_images_by_user_impl(
        &self,
        user_id: Uuid,
    ) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) as count FROM images WHERE user_id = $1 AND deleted_at IS NULL",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0)
    }

    pub(super) async fn find_images_by_user_and_ids_impl(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<Vec<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(&format!(
            "SELECT {} FROM images WHERE user_id = $1 AND id = ANY($2)",
            IMAGE_SELECT_COLUMNS
        ))
        .bind(user_id)
        .bind(image_ids)
        .fetch_all(&self.pool)
        .await
    }

    pub(super) async fn find_images_by_user_and_hashes_impl(
        &self,
        user_id: Uuid,
        image_keys: &[String],
    ) -> Result<Vec<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(&format!(
            "SELECT {} FROM images WHERE user_id = $1 AND hash = ANY($2)",
            IMAGE_SELECT_COLUMNS
        ))
        .bind(user_id)
        .bind(image_keys)
        .fetch_all(&self.pool)
        .await
    }

    pub(super) async fn find_image_by_hash_impl(
        &self,
        hash: &str,
        user_id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(&format!(
            "SELECT {} FROM images WHERE hash = $1 AND user_id = $2 AND deleted_at IS NULL",
            IMAGE_SELECT_COLUMNS
        ))
        .bind(hash)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub(super) async fn find_image_by_hash_global_impl(
        &self,
        hash: &str,
    ) -> Result<Option<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(&format!(
            "SELECT {} FROM images WHERE hash = $1 AND deleted_at IS NULL LIMIT 1",
            IMAGE_SELECT_COLUMNS
        ))
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
    }

    pub(super) async fn find_deleted_images_by_user_impl(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(
            "SELECT * FROM images WHERE user_id = $1 AND deleted_at IS NOT NULL ORDER BY deleted_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    pub(super) async fn find_deleted_images_by_user_paginated_impl(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(&format!(
            "SELECT {} FROM images WHERE user_id = $1 AND deleted_at IS NOT NULL ORDER BY deleted_at DESC LIMIT $2 OFFSET $3",
            IMAGE_SELECT_WITH_TOTAL_COUNT
        ))
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    pub(super) async fn find_images_by_user_cursor_impl(
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
                     LIMIT $4",
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
                     LIMIT $2",
                )
                .bind(user_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
            }
        }
    }
}
