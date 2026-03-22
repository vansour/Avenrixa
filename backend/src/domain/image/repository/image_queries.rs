use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

use super::PostgresImageRepository;
use super::sql::{IMAGE_SELECT_COLUMNS, MEDIA_BLOB_SELECT_COLUMNS};
use crate::models::{Image, ImageStatus, MediaBlob};

fn active_status() -> &'static str {
    ImageStatus::Active.as_str()
}

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

    pub(super) async fn find_images_by_user_after_cursor_impl(
        &self,
        user_id: Uuid,
        limit: i32,
        cursor_created_at: Option<DateTime<Utc>>,
        cursor_id: Option<Uuid>,
    ) -> Result<Vec<Image>, sqlx::Error> {
        let mut builder = QueryBuilder::<Postgres>::new("SELECT ");
        builder.push(IMAGE_SELECT_COLUMNS);
        builder.push(" FROM images WHERE images.user_id = ");
        builder.push_bind(user_id);
        builder.push(" AND images.status = ");
        builder.push_bind(active_status());

        if let (Some(cursor_created_at), Some(cursor_id)) = (cursor_created_at, cursor_id) {
            builder.push(" AND (images.created_at < ");
            builder.push_bind(cursor_created_at);
            builder.push(" OR (images.created_at = ");
            builder.push_bind(cursor_created_at);
            builder.push(" AND images.id < ");
            builder.push_bind(cursor_id);
            builder.push("))");
        }

        builder.push(" ORDER BY images.created_at DESC, images.id DESC");
        builder.push(" LIMIT ");
        builder.push_bind(limit);

        builder
            .build_query_as::<Image>()
            .fetch_all(&self.pool)
            .await
    }

    pub(super) async fn find_images_by_user_and_ids_impl(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<Vec<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(&format!(
            "SELECT {} FROM images WHERE user_id = $1 AND id = ANY($2) AND status = $3",
            IMAGE_SELECT_COLUMNS
        ))
        .bind(user_id)
        .bind(image_ids)
        .bind(active_status())
        .fetch_all(&self.pool)
        .await
    }

    pub(super) async fn find_images_by_user_and_hashes_impl(
        &self,
        user_id: Uuid,
        image_keys: &[String],
    ) -> Result<Vec<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(&format!(
            "SELECT {} FROM images WHERE user_id = $1 AND hash = ANY($2) AND status = $3",
            IMAGE_SELECT_COLUMNS
        ))
        .bind(user_id)
        .bind(image_keys)
        .bind(active_status())
        .fetch_all(&self.pool)
        .await
    }

    pub(super) async fn find_image_by_hash_impl(
        &self,
        hash: &str,
        user_id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(&format!(
            "SELECT {} FROM images WHERE hash = $1 AND user_id = $2 AND status = $3",
            IMAGE_SELECT_COLUMNS
        ))
        .bind(hash)
        .bind(user_id)
        .bind(active_status())
        .fetch_optional(&self.pool)
        .await
    }

    pub(super) async fn find_image_by_hash_global_impl(
        &self,
        hash: &str,
    ) -> Result<Option<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(&format!(
            "SELECT {} FROM images WHERE hash = $1 AND status = $2 LIMIT 1",
            IMAGE_SELECT_COLUMNS
        ))
        .bind(hash)
        .bind(active_status())
        .fetch_optional(&self.pool)
        .await
    }

    pub(super) async fn find_image_by_filename_impl(
        &self,
        filename: &str,
        user_id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(&format!(
            "SELECT {} FROM images WHERE filename = $1 AND user_id = $2 AND status = $3 LIMIT 1",
            IMAGE_SELECT_COLUMNS
        ))
        .bind(filename)
        .bind(user_id)
        .bind(active_status())
        .fetch_optional(&self.pool)
        .await
    }

    pub(super) async fn upsert_media_blob_impl(
        &self,
        storage_key: &str,
        media_kind: &str,
        content_hash: Option<&str>,
    ) -> Result<MediaBlob, sqlx::Error> {
        sqlx::query_as::<_, MediaBlob>(&format!(
            "INSERT INTO media_blobs (
                 storage_key,
                 media_kind,
                 content_hash,
                 ref_count,
                 status,
                 created_at,
                 updated_at
             )
             VALUES ($1, $2, $3, 0, 'ready', NOW(), NOW())
             ON CONFLICT (storage_key) DO UPDATE
             SET
                 media_kind = EXCLUDED.media_kind,
                 content_hash = COALESCE(EXCLUDED.content_hash, media_blobs.content_hash),
                 status = 'ready',
                 updated_at = NOW()
             RETURNING {}",
            MEDIA_BLOB_SELECT_COLUMNS
        ))
        .bind(storage_key)
        .bind(media_kind)
        .bind(content_hash)
        .fetch_one(&self.pool)
        .await
    }

    pub(super) async fn find_media_blobs_by_keys_impl(
        &self,
        storage_keys: &[String],
    ) -> Result<Vec<MediaBlob>, sqlx::Error> {
        if storage_keys.is_empty() {
            return Ok(Vec::new());
        }

        sqlx::query_as::<_, MediaBlob>(&format!(
            "SELECT {} FROM media_blobs WHERE storage_key = ANY($1)",
            MEDIA_BLOB_SELECT_COLUMNS
        ))
        .bind(storage_keys)
        .fetch_all(&self.pool)
        .await
    }

    pub(super) async fn adjust_media_blob_ref_counts_impl(
        &self,
        adjustments: &[(String, i64)],
    ) -> Result<(), sqlx::Error> {
        let mut aggregated = BTreeMap::new();
        for (storage_key, delta) in adjustments {
            *aggregated.entry(storage_key.clone()).or_insert(0_i64) += delta;
        }

        for (storage_key, delta) in aggregated {
            if delta == 0 {
                continue;
            }

            sqlx::query(
                "UPDATE media_blobs
                 SET ref_count = GREATEST(ref_count + $1, 0),
                     status = CASE
                         WHEN ref_count + $1 > 0 THEN 'ready'
                         ELSE status
                     END,
                     updated_at = NOW()
                 WHERE storage_key = $2",
            )
            .bind(delta)
            .bind(storage_key)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub(super) async fn set_media_blob_status_impl(
        &self,
        storage_keys: &[String],
        status: &str,
    ) -> Result<(), sqlx::Error> {
        if storage_keys.is_empty() {
            return Ok(());
        }

        sqlx::query(
            "UPDATE media_blobs
             SET status = $2,
                 updated_at = NOW()
             WHERE storage_key = ANY($1)",
        )
        .bind(storage_keys)
        .bind(status)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
