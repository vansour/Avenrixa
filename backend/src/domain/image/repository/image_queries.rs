use sqlx::{MySql, Postgres, QueryBuilder, Sqlite};
use uuid::Uuid;

use super::sql::{
    IMAGE_SELECT_COLUMNS, IMAGE_SELECT_WITH_TOTAL_COUNT, MYSQL_IMAGE_SELECT_WITH_TOTAL_COUNT,
};
use super::{MySqlImageRepository, PostgresImageRepository, SqliteImageRepository};
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

    pub(super) async fn find_filenames_still_referenced_excluding_ids_impl(
        &self,
        filenames: &[String],
        excluded_ids: &[Uuid],
    ) -> Result<Vec<String>, sqlx::Error> {
        if filenames.is_empty() {
            return Ok(Vec::new());
        }

        sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT filename
             FROM images
             WHERE filename = ANY($1)
               AND NOT (id = ANY($2))",
        )
        .bind(filenames)
        .bind(excluded_ids)
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
}

impl MySqlImageRepository {
    pub(super) async fn find_image_by_id_impl(
        &self,
        id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(&format!(
            "SELECT {} FROM images WHERE id = ?",
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
        tag: Option<&str>,
    ) -> Result<Vec<Image>, sqlx::Error> {
        let tag = tag
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_lowercase());

        let mut builder = QueryBuilder::<MySql>::new("SELECT ");
        builder.push(MYSQL_IMAGE_SELECT_WITH_TOTAL_COUNT);
        builder.push(" FROM images WHERE images.user_id = ");
        builder.push_bind(user_id);
        builder.push(" AND images.deleted_at IS NULL AND images.status = 'active'");

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

    pub(super) async fn find_images_by_user_and_ids_impl(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<Vec<Image>, sqlx::Error> {
        if image_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut builder = QueryBuilder::<MySql>::new("SELECT ");
        builder.push(IMAGE_SELECT_COLUMNS);
        builder.push(" FROM images WHERE user_id = ");
        builder.push_bind(user_id);
        builder.push(" AND id IN (");
        {
            let mut separated = builder.separated(", ");
            for image_id in image_ids {
                separated.push_bind(image_id);
            }
        }
        builder.push(")");

        builder
            .build_query_as::<Image>()
            .fetch_all(&self.pool)
            .await
    }

    pub(super) async fn find_images_by_user_and_hashes_impl(
        &self,
        user_id: Uuid,
        image_keys: &[String],
    ) -> Result<Vec<Image>, sqlx::Error> {
        if image_keys.is_empty() {
            return Ok(Vec::new());
        }

        let mut builder = QueryBuilder::<MySql>::new("SELECT ");
        builder.push(IMAGE_SELECT_COLUMNS);
        builder.push(" FROM images WHERE user_id = ");
        builder.push_bind(user_id);
        builder.push(" AND hash IN (");
        {
            let mut separated = builder.separated(", ");
            for image_key in image_keys {
                separated.push_bind(image_key);
            }
        }
        builder.push(")");

        builder
            .build_query_as::<Image>()
            .fetch_all(&self.pool)
            .await
    }

    pub(super) async fn find_filenames_still_referenced_excluding_ids_impl(
        &self,
        filenames: &[String],
        excluded_ids: &[Uuid],
    ) -> Result<Vec<String>, sqlx::Error> {
        if filenames.is_empty() {
            return Ok(Vec::new());
        }

        let mut builder =
            QueryBuilder::<MySql>::new("SELECT DISTINCT filename FROM images WHERE filename IN (");
        {
            let mut separated = builder.separated(", ");
            for filename in filenames {
                separated.push_bind(filename);
            }
        }
        builder.push(")");

        if !excluded_ids.is_empty() {
            builder.push(" AND id NOT IN (");
            {
                let mut separated = builder.separated(", ");
                for image_id in excluded_ids {
                    separated.push_bind(image_id);
                }
            }
            builder.push(")");
        }

        builder.build_query_scalar().fetch_all(&self.pool).await
    }

    pub(super) async fn find_image_by_hash_impl(
        &self,
        hash: &str,
        user_id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(&format!(
            "SELECT {} FROM images WHERE hash = ? AND user_id = ? AND deleted_at IS NULL",
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
            "SELECT {} FROM images WHERE hash = ? AND deleted_at IS NULL LIMIT 1",
            IMAGE_SELECT_COLUMNS
        ))
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
    }
}

impl SqliteImageRepository {
    pub(super) async fn find_image_by_id_impl(
        &self,
        id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(&format!(
            "SELECT {} FROM images WHERE id = ?1",
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
        tag: Option<&str>,
    ) -> Result<Vec<Image>, sqlx::Error> {
        let tag = tag
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_lowercase());

        let mut builder = QueryBuilder::<Sqlite>::new("SELECT ");
        builder.push(IMAGE_SELECT_WITH_TOTAL_COUNT);
        builder.push(" FROM images WHERE images.user_id = ");
        builder.push_bind(user_id);
        builder.push(" AND images.deleted_at IS NULL AND images.status = 'active'");

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

    pub(super) async fn find_images_by_user_and_ids_impl(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<Vec<Image>, sqlx::Error> {
        if image_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut builder = QueryBuilder::<Sqlite>::new("SELECT ");
        builder.push(IMAGE_SELECT_COLUMNS);
        builder.push(" FROM images WHERE user_id = ");
        builder.push_bind(user_id);
        builder.push(" AND id IN (");
        {
            let mut separated = builder.separated(", ");
            for image_id in image_ids {
                separated.push_bind(image_id);
            }
        }
        builder.push(")");

        builder
            .build_query_as::<Image>()
            .fetch_all(&self.pool)
            .await
    }

    pub(super) async fn find_images_by_user_and_hashes_impl(
        &self,
        user_id: Uuid,
        image_keys: &[String],
    ) -> Result<Vec<Image>, sqlx::Error> {
        if image_keys.is_empty() {
            return Ok(Vec::new());
        }

        let mut builder = QueryBuilder::<Sqlite>::new("SELECT ");
        builder.push(IMAGE_SELECT_COLUMNS);
        builder.push(" FROM images WHERE user_id = ");
        builder.push_bind(user_id);
        builder.push(" AND hash IN (");
        {
            let mut separated = builder.separated(", ");
            for image_key in image_keys {
                separated.push_bind(image_key);
            }
        }
        builder.push(")");

        builder
            .build_query_as::<Image>()
            .fetch_all(&self.pool)
            .await
    }

    pub(super) async fn find_filenames_still_referenced_excluding_ids_impl(
        &self,
        filenames: &[String],
        excluded_ids: &[Uuid],
    ) -> Result<Vec<String>, sqlx::Error> {
        if filenames.is_empty() {
            return Ok(Vec::new());
        }

        let mut builder =
            QueryBuilder::<Sqlite>::new("SELECT DISTINCT filename FROM images WHERE filename IN (");
        {
            let mut separated = builder.separated(", ");
            for filename in filenames {
                separated.push_bind(filename);
            }
        }
        builder.push(")");

        if !excluded_ids.is_empty() {
            builder.push(" AND id NOT IN (");
            {
                let mut separated = builder.separated(", ");
                for image_id in excluded_ids {
                    separated.push_bind(image_id);
                }
            }
            builder.push(")");
        }

        builder.build_query_scalar().fetch_all(&self.pool).await
    }

    pub(super) async fn find_image_by_hash_impl(
        &self,
        hash: &str,
        user_id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        sqlx::query_as::<_, Image>(&format!(
            "SELECT {} FROM images WHERE hash = ?1 AND user_id = ?2 AND deleted_at IS NULL",
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
            "SELECT {} FROM images WHERE hash = ?1 AND deleted_at IS NULL LIMIT 1",
            IMAGE_SELECT_COLUMNS
        ))
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
    }
}
