use std::collections::BTreeSet;

use sqlx::{MySql, Postgres, QueryBuilder, Sqlite};
use uuid::Uuid;

use super::sql::{
    IMAGE_SELECT_COLUMNS, IMAGE_SELECT_WITH_TOTAL_COUNT, MYSQL_IMAGE_SELECT_WITH_TOTAL_COUNT,
};
use super::{MySqlImageRepository, PostgresImageRepository, SqliteImageRepository};
use crate::models::{Image, ImageStatus};

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

    pub(super) async fn find_images_by_user_paginated_impl(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<Image>, sqlx::Error> {
        let mut builder = QueryBuilder::<Postgres>::new("SELECT ");
        builder.push(IMAGE_SELECT_WITH_TOTAL_COUNT);
        builder.push(" FROM images WHERE images.user_id = ");
        builder.push_bind(user_id);
        builder.push(" AND images.status = ");
        builder.push_bind(active_status());

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
               AND status = $3
               AND NOT (id = ANY($2))",
        )
        .bind(filenames)
        .bind(excluded_ids)
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

    pub(super) async fn find_media_keys_still_referenced_excluding_ids_impl(
        &self,
        media_keys: &[String],
        excluded_ids: &[Uuid],
    ) -> Result<Vec<String>, sqlx::Error> {
        if media_keys.is_empty() {
            return Ok(Vec::new());
        }

        let mut referenced = BTreeSet::new();
        let filename_refs = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT filename
             FROM images
             WHERE filename = ANY($1)
               AND status = $2
               AND NOT (id = ANY($3))",
        )
        .bind(media_keys)
        .bind(active_status())
        .bind(excluded_ids)
        .fetch_all(&self.pool)
        .await?;
        referenced.extend(filename_refs);

        let thumbnail_refs = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT thumbnail
             FROM images
             WHERE thumbnail = ANY($1)
               AND status = $2
               AND thumbnail IS NOT NULL
               AND NOT (id = ANY($3))",
        )
        .bind(media_keys)
        .bind(active_status())
        .bind(excluded_ids)
        .fetch_all(&self.pool)
        .await?;
        referenced.extend(thumbnail_refs);

        Ok(referenced.into_iter().collect())
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
    ) -> Result<Vec<Image>, sqlx::Error> {
        let mut builder = QueryBuilder::<MySql>::new("SELECT ");
        builder.push(MYSQL_IMAGE_SELECT_WITH_TOTAL_COUNT);
        builder.push(" FROM images WHERE images.user_id = ");
        builder.push_bind(user_id);
        builder.push(" AND images.status = ");
        builder.push_bind(active_status());

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
        builder.push(" AND status = ");
        builder.push_bind(active_status());
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
        builder.push(" AND status = ");
        builder.push_bind(active_status());
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
        builder.push(" AND status = ");
        builder.push_bind(active_status());

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
            "SELECT {} FROM images WHERE hash = ? AND user_id = ? AND status = ?",
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
            "SELECT {} FROM images WHERE hash = ? AND status = ? LIMIT 1",
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
            "SELECT {} FROM images WHERE filename = ? AND user_id = ? AND status = ? LIMIT 1",
            IMAGE_SELECT_COLUMNS
        ))
        .bind(filename)
        .bind(user_id)
        .bind(active_status())
        .fetch_optional(&self.pool)
        .await
    }

    pub(super) async fn find_media_keys_still_referenced_excluding_ids_impl(
        &self,
        media_keys: &[String],
        excluded_ids: &[Uuid],
    ) -> Result<Vec<String>, sqlx::Error> {
        if media_keys.is_empty() {
            return Ok(Vec::new());
        }

        let mut referenced = BTreeSet::new();
        referenced.extend(
            mysql_media_refs_query(media_keys, excluded_ids, "filename")
                .build_query_scalar::<String>()
                .fetch_all(&self.pool)
                .await?,
        );
        referenced.extend(
            mysql_media_refs_query(media_keys, excluded_ids, "thumbnail")
                .build_query_scalar::<String>()
                .fetch_all(&self.pool)
                .await?,
        );

        Ok(referenced.into_iter().collect())
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
    ) -> Result<Vec<Image>, sqlx::Error> {
        let mut builder = QueryBuilder::<Sqlite>::new("SELECT ");
        builder.push(IMAGE_SELECT_WITH_TOTAL_COUNT);
        builder.push(" FROM images WHERE images.user_id = ");
        builder.push_bind(user_id);
        builder.push(" AND images.status = ");
        builder.push_bind(active_status());

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
        builder.push(" AND status = ");
        builder.push_bind(active_status());
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
        builder.push(" AND status = ");
        builder.push_bind(active_status());
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
        builder.push(" AND status = ");
        builder.push_bind(active_status());

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
            "SELECT {} FROM images WHERE hash = ?1 AND user_id = ?2 AND status = ?3",
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
            "SELECT {} FROM images WHERE hash = ?1 AND status = ?2 LIMIT 1",
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
            "SELECT {} FROM images WHERE filename = ?1 AND user_id = ?2 AND status = ?3 LIMIT 1",
            IMAGE_SELECT_COLUMNS
        ))
        .bind(filename)
        .bind(user_id)
        .bind(active_status())
        .fetch_optional(&self.pool)
        .await
    }

    pub(super) async fn find_media_keys_still_referenced_excluding_ids_impl(
        &self,
        media_keys: &[String],
        excluded_ids: &[Uuid],
    ) -> Result<Vec<String>, sqlx::Error> {
        if media_keys.is_empty() {
            return Ok(Vec::new());
        }

        let mut referenced = BTreeSet::new();
        referenced.extend(
            sqlite_media_refs_query(media_keys, excluded_ids, "filename")
                .build_query_scalar::<String>()
                .fetch_all(&self.pool)
                .await?,
        );
        referenced.extend(
            sqlite_media_refs_query(media_keys, excluded_ids, "thumbnail")
                .build_query_scalar::<String>()
                .fetch_all(&self.pool)
                .await?,
        );

        Ok(referenced.into_iter().collect())
    }
}

fn mysql_media_refs_query<'a>(
    media_keys: &'a [String],
    excluded_ids: &'a [Uuid],
    column: &'a str,
) -> QueryBuilder<'a, MySql> {
    let mut builder = QueryBuilder::<MySql>::new(format!(
        "SELECT DISTINCT {column} FROM images WHERE {column} IN ("
    ));
    {
        let mut separated = builder.separated(", ");
        for media_key in media_keys {
            separated.push_bind(media_key);
        }
    }
    builder.push(")");
    builder.push(format!(" AND {column} IS NOT NULL"));
    builder.push(" AND status = ");
    builder.push_bind(active_status());

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

    builder
}

fn sqlite_media_refs_query<'a>(
    media_keys: &'a [String],
    excluded_ids: &'a [Uuid],
    column: &'a str,
) -> QueryBuilder<'a, Sqlite> {
    let mut builder = QueryBuilder::<Sqlite>::new(format!(
        "SELECT DISTINCT {column} FROM images WHERE {column} IN ("
    ));
    {
        let mut separated = builder.separated(", ");
        for media_key in media_keys {
            separated.push_bind(media_key);
        }
    }
    builder.push(")");
    builder.push(format!(" AND {column} IS NOT NULL"));
    builder.push(" AND status = ");
    builder.push_bind(active_status());

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

    builder
}
