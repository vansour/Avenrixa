use async_trait::async_trait;
use uuid::Uuid;

use super::{
    DatabaseImageRepository, ImageRepository, PostgresImageRepository, SqliteImageRepository,
};
use crate::models::Image;

#[async_trait]
impl ImageRepository for PostgresImageRepository {
    async fn find_image_by_id(&self, id: Uuid) -> Result<Option<Image>, sqlx::Error> {
        self.find_image_by_id_impl(id).await
    }

    async fn find_images_by_user_paginated(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
        tag: Option<&str>,
    ) -> Result<Vec<Image>, sqlx::Error> {
        self.find_images_by_user_paginated_impl(user_id, limit, offset, tag)
            .await
    }

    async fn create_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        self.create_image_impl(image).await
    }

    async fn update_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        self.update_image_impl(image).await
    }

    async fn soft_delete_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        self.soft_delete_images_by_user_impl(user_id, image_ids)
            .await
    }

    async fn restore_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        self.restore_images_by_user_impl(user_id, image_ids).await
    }

    async fn find_images_by_user_and_ids(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<Vec<Image>, sqlx::Error> {
        self.find_images_by_user_and_ids_impl(user_id, image_ids)
            .await
    }

    async fn find_images_by_user_and_hashes(
        &self,
        user_id: Uuid,
        image_keys: &[String],
    ) -> Result<Vec<Image>, sqlx::Error> {
        self.find_images_by_user_and_hashes_impl(user_id, image_keys)
            .await
    }

    async fn hard_delete_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        self.hard_delete_images_by_user_impl(user_id, image_ids)
            .await
    }

    async fn find_filenames_still_referenced_excluding_ids(
        &self,
        filenames: &[String],
        excluded_ids: &[Uuid],
    ) -> Result<Vec<String>, sqlx::Error> {
        self.find_filenames_still_referenced_excluding_ids_impl(filenames, excluded_ids)
            .await
    }

    async fn find_image_by_hash(
        &self,
        hash: &str,
        user_id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        self.find_image_by_hash_impl(hash, user_id).await
    }

    async fn find_image_by_hash_global(&self, hash: &str) -> Result<Option<Image>, sqlx::Error> {
        self.find_image_by_hash_global_impl(hash).await
    }

    async fn find_deleted_images_by_user_paginated(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<Image>, sqlx::Error> {
        self.find_deleted_images_by_user_paginated_impl(user_id, limit, offset)
            .await
    }
}

#[async_trait]
impl ImageRepository for SqliteImageRepository {
    async fn find_image_by_id(&self, id: Uuid) -> Result<Option<Image>, sqlx::Error> {
        self.find_image_by_id_impl(id).await
    }

    async fn find_images_by_user_paginated(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
        tag: Option<&str>,
    ) -> Result<Vec<Image>, sqlx::Error> {
        self.find_images_by_user_paginated_impl(user_id, limit, offset, tag)
            .await
    }

    async fn create_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        self.create_image_impl(image).await
    }

    async fn update_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        self.update_image_impl(image).await
    }

    async fn soft_delete_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        self.soft_delete_images_by_user_impl(user_id, image_ids)
            .await
    }

    async fn restore_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        self.restore_images_by_user_impl(user_id, image_ids).await
    }

    async fn find_images_by_user_and_ids(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<Vec<Image>, sqlx::Error> {
        self.find_images_by_user_and_ids_impl(user_id, image_ids)
            .await
    }

    async fn find_images_by_user_and_hashes(
        &self,
        user_id: Uuid,
        image_keys: &[String],
    ) -> Result<Vec<Image>, sqlx::Error> {
        self.find_images_by_user_and_hashes_impl(user_id, image_keys)
            .await
    }

    async fn hard_delete_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        self.hard_delete_images_by_user_impl(user_id, image_ids)
            .await
    }

    async fn find_filenames_still_referenced_excluding_ids(
        &self,
        filenames: &[String],
        excluded_ids: &[Uuid],
    ) -> Result<Vec<String>, sqlx::Error> {
        self.find_filenames_still_referenced_excluding_ids_impl(filenames, excluded_ids)
            .await
    }

    async fn find_image_by_hash(
        &self,
        hash: &str,
        user_id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        self.find_image_by_hash_impl(hash, user_id).await
    }

    async fn find_image_by_hash_global(&self, hash: &str) -> Result<Option<Image>, sqlx::Error> {
        self.find_image_by_hash_global_impl(hash).await
    }

    async fn find_deleted_images_by_user_paginated(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<Image>, sqlx::Error> {
        self.find_deleted_images_by_user_paginated_impl(user_id, limit, offset)
            .await
    }
}

#[async_trait]
impl ImageRepository for DatabaseImageRepository {
    async fn find_image_by_id(&self, id: Uuid) -> Result<Option<Image>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.find_image_by_id(id).await,
            Self::Sqlite(repo) => repo.find_image_by_id(id).await,
        }
    }

    async fn find_images_by_user_paginated(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
        tag: Option<&str>,
    ) -> Result<Vec<Image>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => {
                repo.find_images_by_user_paginated(user_id, limit, offset, tag)
                    .await
            }
            Self::Sqlite(repo) => {
                repo.find_images_by_user_paginated(user_id, limit, offset, tag)
                    .await
            }
        }
    }

    async fn create_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.create_image(image).await,
            Self::Sqlite(repo) => repo.create_image(image).await,
        }
    }

    async fn update_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.update_image(image).await,
            Self::Sqlite(repo) => repo.update_image(image).await,
        }
    }

    async fn soft_delete_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.soft_delete_images_by_user(user_id, image_ids).await,
            Self::Sqlite(repo) => repo.soft_delete_images_by_user(user_id, image_ids).await,
        }
    }

    async fn restore_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.restore_images_by_user(user_id, image_ids).await,
            Self::Sqlite(repo) => repo.restore_images_by_user(user_id, image_ids).await,
        }
    }

    async fn find_images_by_user_and_ids(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<Vec<Image>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.find_images_by_user_and_ids(user_id, image_ids).await,
            Self::Sqlite(repo) => repo.find_images_by_user_and_ids(user_id, image_ids).await,
        }
    }

    async fn find_images_by_user_and_hashes(
        &self,
        user_id: Uuid,
        image_keys: &[String],
    ) -> Result<Vec<Image>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => {
                repo.find_images_by_user_and_hashes(user_id, image_keys)
                    .await
            }
            Self::Sqlite(repo) => {
                repo.find_images_by_user_and_hashes(user_id, image_keys)
                    .await
            }
        }
    }

    async fn find_filenames_still_referenced_excluding_ids(
        &self,
        filenames: &[String],
        excluded_ids: &[Uuid],
    ) -> Result<Vec<String>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => {
                repo.find_filenames_still_referenced_excluding_ids(filenames, excluded_ids)
                    .await
            }
            Self::Sqlite(repo) => {
                repo.find_filenames_still_referenced_excluding_ids(filenames, excluded_ids)
                    .await
            }
        }
    }

    async fn hard_delete_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.hard_delete_images_by_user(user_id, image_ids).await,
            Self::Sqlite(repo) => repo.hard_delete_images_by_user(user_id, image_ids).await,
        }
    }

    async fn find_image_by_hash(
        &self,
        hash: &str,
        user_id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.find_image_by_hash(hash, user_id).await,
            Self::Sqlite(repo) => repo.find_image_by_hash(hash, user_id).await,
        }
    }

    async fn find_image_by_hash_global(&self, hash: &str) -> Result<Option<Image>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.find_image_by_hash_global(hash).await,
            Self::Sqlite(repo) => repo.find_image_by_hash_global(hash).await,
        }
    }

    async fn find_deleted_images_by_user_paginated(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<Image>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => {
                repo.find_deleted_images_by_user_paginated(user_id, limit, offset)
                    .await
            }
            Self::Sqlite(repo) => {
                repo.find_deleted_images_by_user_paginated(user_id, limit, offset)
                    .await
            }
        }
    }
}
