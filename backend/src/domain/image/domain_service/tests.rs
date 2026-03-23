use super::*;
use crate::config::Config;
use crate::db::DatabasePool;
use crate::domain::image::mock_repository::MockImageRepository;
use crate::image_processor::ImageProcessor;
use crate::models::{ImageStatus, MediaBlob};
use crate::observability::RuntimeObservability;
use crate::runtime_settings::{RuntimeSettings, StorageBackend};
use crate::storage_backend::StorageManager;
use async_trait::async_trait;
use std::sync::Arc;
use tempfile::TempDir;

struct TestServiceContext<I: ImageRepository> {
    service: ImageDomainService<I>,
    _temp_dir: TempDir,
}

fn sample_runtime_settings(local_storage_path: String) -> RuntimeSettings {
    RuntimeSettings {
        site_name: "Avenrixa".to_string(),
        storage_backend: StorageBackend::Local,
        local_storage_path,
        mail_enabled: false,
        mail_smtp_host: "smtp.example.com".to_string(),
        mail_smtp_port: 587,
        mail_smtp_user: None,
        mail_smtp_password: None,
        mail_from_email: "noreply@example.com".to_string(),
        mail_from_name: "Avenrixa".to_string(),
        mail_link_base_url: "https://img.example.com".to_string(),
    }
}

async fn setup_service_with_repository<I: ImageRepository>(
    database: DatabasePool,
    image_repository: I,
) -> TestServiceContext<I> {
    let mut config = Config::default();
    let image_processor = ImageProcessor::new(1920, 1080, 80);

    let cache = None;
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let local_storage_path = temp_dir.path().join("images");
    config.storage.path = local_storage_path.to_string_lossy().into_owned();

    let storage_manager = Arc::new(StorageManager::new(sample_runtime_settings(
        config.storage.path.clone(),
    )));
    let dependencies = ImageDomainServiceDependencies::new(
        database,
        cache,
        config,
        image_processor,
        storage_manager,
        Arc::new(RuntimeObservability::new()),
    );

    TestServiceContext {
        service: ImageDomainService::new(dependencies, image_repository),
        _temp_dir: temp_dir,
    }
}

async fn setup_service() -> TestServiceContext<MockImageRepository> {
    let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").unwrap();
    setup_service_with_repository(DatabasePool::Postgres(pool), MockImageRepository::new()).await
}

async fn storage_entry_count<I: ImageRepository>(service: &ImageDomainService<I>) -> usize {
    let storage_path = std::path::Path::new(&service.config.storage.path);
    if !tokio::fs::try_exists(storage_path)
        .await
        .expect("storage path existence check should succeed")
    {
        return 0;
    }

    let mut entries = tokio::fs::read_dir(storage_path)
        .await
        .expect("storage dir should be readable");
    let mut count = 0;
    while entries
        .next_entry()
        .await
        .expect("storage dir iteration should succeed")
        .is_some()
    {
        count += 1;
    }
    count
}

struct FaultyImageRepository {
    inner: MockImageRepository,
    fail_on_create: bool,
    fail_on_hard_delete: bool,
}

impl FaultyImageRepository {
    fn fail_create() -> Self {
        Self {
            inner: MockImageRepository::new(),
            fail_on_create: true,
            fail_on_hard_delete: false,
        }
    }

    fn fail_hard_delete() -> Self {
        Self {
            inner: MockImageRepository::new(),
            fail_on_create: false,
            fail_on_hard_delete: true,
        }
    }
}

#[async_trait]
impl ImageRepository for FaultyImageRepository {
    async fn find_image_by_id(&self, id: Uuid) -> Result<Option<Image>, sqlx::Error> {
        self.inner.find_image_by_id(id).await
    }

    async fn find_images_by_user_after_cursor(
        &self,
        user_id: Uuid,
        limit: i32,
        cursor_created_at: Option<chrono::DateTime<Utc>>,
        cursor_id: Option<Uuid>,
    ) -> Result<Vec<Image>, sqlx::Error> {
        self.inner
            .find_images_by_user_after_cursor(user_id, limit, cursor_created_at, cursor_id)
            .await
    }

    async fn create_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        if self.fail_on_create {
            return Err(sqlx::Error::RowNotFound);
        }
        self.inner.create_image(image).await
    }

    async fn update_image(&self, image: &Image) -> Result<(), sqlx::Error> {
        self.inner.update_image(image).await
    }

    async fn find_images_by_user_and_ids(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<Vec<Image>, sqlx::Error> {
        self.inner
            .find_images_by_user_and_ids(user_id, image_ids)
            .await
    }

    async fn find_images_by_user_and_hashes(
        &self,
        user_id: Uuid,
        image_keys: &[String],
    ) -> Result<Vec<Image>, sqlx::Error> {
        self.inner
            .find_images_by_user_and_hashes(user_id, image_keys)
            .await
    }

    async fn hard_delete_images_by_user(
        &self,
        user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error> {
        if self.fail_on_hard_delete {
            return Err(sqlx::Error::RowNotFound);
        }
        self.inner
            .hard_delete_images_by_user(user_id, image_ids)
            .await
    }

    async fn find_image_by_hash(
        &self,
        hash: &str,
        user_id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        self.inner.find_image_by_hash(hash, user_id).await
    }

    async fn find_image_by_hash_global(&self, hash: &str) -> Result<Option<Image>, sqlx::Error> {
        self.inner.find_image_by_hash_global(hash).await
    }

    async fn upsert_media_blob(
        &self,
        storage_key: &str,
        media_kind: &str,
        content_hash: Option<&str>,
    ) -> Result<MediaBlob, sqlx::Error> {
        self.inner
            .upsert_media_blob(storage_key, media_kind, content_hash)
            .await
    }

    async fn find_media_blobs_by_keys(
        &self,
        storage_keys: &[String],
    ) -> Result<Vec<MediaBlob>, sqlx::Error> {
        self.inner.find_media_blobs_by_keys(storage_keys).await
    }

    async fn adjust_media_blob_ref_counts(
        &self,
        adjustments: &[(String, i64)],
    ) -> Result<(), sqlx::Error> {
        self.inner.adjust_media_blob_ref_counts(adjustments).await
    }

    async fn set_media_blob_status(
        &self,
        storage_keys: &[String],
        status: &str,
    ) -> Result<(), sqlx::Error> {
        self.inner.set_media_blob_status(storage_keys, status).await
    }

    async fn find_image_by_filename(
        &self,
        filename: &str,
        user_id: Uuid,
    ) -> Result<Option<Image>, sqlx::Error> {
        self.inner.find_image_by_filename(filename, user_id).await
    }
}

fn valid_hash(seed: u64) -> String {
    format!("{seed:064x}")
}

fn sample_png_bytes() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]));
    let mut cursor = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut cursor, image::ImageFormat::Png)
        .expect("png encoding should succeed");
    cursor.into_inner()
}

fn build_image(
    id: Uuid,
    user_id: Uuid,
    filename: &str,
    hash: &str,
    created_at: chrono::DateTime<Utc>,
) -> Image {
    Image {
        id,
        user_id,
        filename: filename.to_string(),
        thumbnail: None,
        size: 100,
        hash: hash.to_string(),
        format: "jpg".to_string(),
        views: 0,
        status: ImageStatus::Active,
        expires_at: None,
        created_at,
        total_count: None,
    }
}

#[tokio::test]
async fn test_get_image_not_found() {
    let service = setup_service().await.service;
    let user_id = Uuid::new_v4();
    let image_id = Uuid::new_v4();
    let result = service.get_image_by_id(image_id, user_id).await;
    assert!(matches!(result, Err(AppError::ImageNotFound)));
}

#[tokio::test]
async fn test_set_expiry_updates_owned_image() {
    let context = setup_service().await;
    let service = &context.service;
    let user_id = Uuid::new_v4();
    let image_id = Uuid::new_v4();
    let image = build_image(image_id, user_id, "expires.jpg", &valid_hash(1), Utc::now());
    let expires_at = Utc::now() + chrono::Duration::days(7);

    service.image_repository.create_image(&image).await.unwrap();
    service
        .set_expiry(image_id, user_id, Some(expires_at))
        .await
        .expect("expiry update should succeed");

    let updated = service
        .get_image_by_id(image_id, user_id)
        .await
        .expect("image should remain accessible");
    assert_eq!(updated.expires_at, Some(expires_at));
}

#[tokio::test]
async fn test_set_expiry_by_key_updates_owned_image() {
    let context = setup_service().await;
    let service = &context.service;
    let user_id = Uuid::new_v4();
    let hash = valid_hash(2);
    let image = build_image(
        Uuid::new_v4(),
        user_id,
        "expires-key.jpg",
        &hash,
        Utc::now(),
    );
    let expires_at = Utc::now() + chrono::Duration::hours(12);

    service.image_repository.create_image(&image).await.unwrap();
    service
        .set_expiry_by_key(&hash, user_id, Some(expires_at))
        .await
        .expect("expiry update by key should succeed");

    let updated = service
        .get_image_by_id(image.id, user_id)
        .await
        .expect("image should remain accessible");
    assert_eq!(updated.expires_at, Some(expires_at));
}

#[tokio::test]
async fn test_set_expiry_rejects_foreign_image() {
    let context = setup_service().await;
    let service = &context.service;
    let owner_id = Uuid::new_v4();
    let other_user_id = Uuid::new_v4();
    let image = build_image(
        Uuid::new_v4(),
        owner_id,
        "foreign.jpg",
        &valid_hash(3),
        Utc::now(),
    );

    service.image_repository.create_image(&image).await.unwrap();

    let error = service
        .set_expiry(image.id, other_user_id, Some(Utc::now()))
        .await
        .expect_err("foreign user should not update expiry");

    assert!(matches!(error, AppError::ImageNotFound));
}

#[tokio::test]
async fn test_increment_views() {
    let service = setup_service().await.service;
    let user_id = Uuid::new_v4();
    let image_id = Uuid::new_v4();

    let image = Image {
        id: image_id,
        user_id,
        filename: "test.jpg".to_string(),
        thumbnail: None,
        size: 100,
        hash: valid_hash(4),
        format: "jpg".to_string(),
        views: 0,
        status: ImageStatus::Active,
        expires_at: None,
        created_at: Utc::now(),
        total_count: None,
    };

    service.image_repository.create_image(&image).await.unwrap();
    let updated = service.increment_views(image_id, user_id).await.unwrap();
    assert_eq!(updated.views, 1);
}

#[tokio::test]
async fn test_list_images_ordered_by_created_at_desc() {
    let service = setup_service().await.service;
    let user_id = Uuid::new_v4();
    let newer = build_image(
        Uuid::new_v4(),
        user_id,
        "newer.jpg",
        &valid_hash(5),
        Utc::now(),
    );
    let older = build_image(
        Uuid::new_v4(),
        user_id,
        "older.jpg",
        &valid_hash(6),
        Utc::now() - chrono::Duration::days(1),
    );

    service.image_repository.create_image(&older).await.unwrap();
    service.image_repository.create_image(&newer).await.unwrap();

    let page = service.get_images(user_id, None, 20).await.unwrap();

    assert_eq!(page.data.len(), 2);
    assert_eq!(page.data[0].id, newer.id);
    assert_eq!(page.data[1].id, older.id);
}

#[tokio::test]
async fn test_list_images_ignores_non_active_status() {
    let service = setup_service().await.service;
    let user_id = Uuid::new_v4();
    let mut hidden = build_image(
        Uuid::new_v4(),
        user_id,
        "hidden.jpg",
        &valid_hash(7),
        Utc::now(),
    );
    hidden.status = ImageStatus::Deleted;
    let visible = build_image(
        Uuid::new_v4(),
        user_id,
        "visible.jpg",
        &valid_hash(8),
        Utc::now() - chrono::Duration::minutes(1),
    );

    service
        .image_repository
        .create_image(&hidden)
        .await
        .unwrap();
    service
        .image_repository
        .create_image(&visible)
        .await
        .unwrap();

    let page = service.get_images(user_id, None, 20).await.unwrap();

    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].id, visible.id);
}

#[tokio::test]
async fn test_list_images_uses_cursor_pagination() {
    let service = setup_service().await.service;
    let user_id = Uuid::new_v4();
    let newest = build_image(
        Uuid::new_v4(),
        user_id,
        "newest.jpg",
        &valid_hash(80),
        Utc::now(),
    );
    let middle = build_image(
        Uuid::new_v4(),
        user_id,
        "middle.jpg",
        &valid_hash(81),
        Utc::now() - chrono::Duration::minutes(1),
    );
    let oldest = build_image(
        Uuid::new_v4(),
        user_id,
        "oldest.jpg",
        &valid_hash(82),
        Utc::now() - chrono::Duration::minutes(2),
    );

    service
        .image_repository
        .create_image(&oldest)
        .await
        .unwrap();
    service
        .image_repository
        .create_image(&middle)
        .await
        .unwrap();
    service
        .image_repository
        .create_image(&newest)
        .await
        .unwrap();

    let first_page = service.get_images(user_id, None, 2).await.unwrap();
    assert_eq!(first_page.data.len(), 2);
    assert_eq!(first_page.data[0].id, newest.id);
    assert_eq!(first_page.data[1].id, middle.id);
    assert!(first_page.has_next);

    let second_page = service
        .get_images(user_id, first_page.next_cursor.as_deref(), 2)
        .await
        .unwrap();
    assert_eq!(second_page.data.len(), 1);
    assert_eq!(second_page.data[0].id, oldest.id);
    assert!(!second_page.has_next);
}

#[tokio::test]
async fn test_bulk_delete_removes_images_from_active_list() {
    let service = setup_service().await.service;
    let user_id = Uuid::new_v4();
    let image_a = build_image(Uuid::new_v4(), user_id, "a.jpg", &valid_hash(9), Utc::now());
    let image_b = build_image(
        Uuid::new_v4(),
        user_id,
        "b.jpg",
        &valid_hash(10),
        Utc::now() - chrono::Duration::minutes(5),
    );

    service
        .image_repository
        .create_image(&image_a)
        .await
        .unwrap();
    service
        .image_repository
        .create_image(&image_b)
        .await
        .unwrap();

    service
        .delete_images(&[image_a.id, image_b.id], user_id)
        .await
        .unwrap();

    let active_images = service.get_images(user_id, None, 20).await.unwrap();
    assert!(active_images.data.is_empty());
    assert!(!active_images.has_next);
}

#[tokio::test]
async fn test_delete_images_by_keys_rejects_invalid_key() {
    let service = setup_service().await.service;
    let user_id = Uuid::new_v4();

    let result = service
        .delete_images_by_keys(&["invalid-key".to_string()], user_id)
        .await;

    assert!(matches!(
        result,
        Err(AppError::ValidationError(message))
            if message == "图片键无效，必须是 64 位十六进制哈希"
    ));
}

#[tokio::test]
async fn test_cross_user_duplicate_upload_creates_second_record_without_filename_conflict() {
    let context = setup_service().await;
    let service = &context.service;
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();
    let payload = sample_png_bytes();

    let first = service
        .upload_image(
            user_a,
            "alice",
            "shared.png".to_string(),
            payload.clone(),
            Some("image/png".to_string()),
        )
        .await
        .expect("first upload should succeed");
    let second = service
        .upload_image(
            user_b,
            "bob",
            "shared.png".to_string(),
            payload,
            Some("image/png".to_string()),
        )
        .await
        .expect("second upload should succeed");

    assert_ne!(first.id, second.id);
    assert_eq!(first.hash, second.hash);
    assert_eq!(first.filename, second.filename);

    let images = service.image_repository.images.lock().unwrap();
    assert_eq!(images.len(), 2);
    assert!(images.iter().any(|image| image.user_id == user_a));
    assert!(images.iter().any(|image| image.user_id == user_b));
}

#[tokio::test]
async fn test_upload_generates_persistent_thumbnail() {
    let context = setup_service().await;
    let service = &context.service;
    let user_id = Uuid::new_v4();

    let image = service
        .upload_image(
            user_id,
            "alice",
            "with-thumb.png".to_string(),
            sample_png_bytes(),
            Some("image/png".to_string()),
        )
        .await
        .expect("upload should succeed");

    let thumbnail_key = image
        .thumbnail
        .clone()
        .expect("uploaded image should have thumbnail key");
    assert!(
        thumbnail_key.ends_with(".webp"),
        "thumbnail should use webp derivative"
    );
    assert!(
        tokio::fs::try_exists(
            std::path::Path::new(&service.config.storage.path).join(&thumbnail_key)
        )
        .await
        .expect("thumbnail file existence check should succeed")
    );
}

#[tokio::test]
async fn test_upload_compensates_storage_when_metadata_persistence_fails() {
    let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").unwrap();
    let context = setup_service_with_repository(
        DatabasePool::Postgres(pool),
        FaultyImageRepository::fail_create(),
    )
    .await;
    let service = &context.service;
    let user_id = Uuid::new_v4();

    let error = service
        .upload_image(
            user_id,
            "alice",
            "broken.png".to_string(),
            sample_png_bytes(),
            Some("image/png".to_string()),
        )
        .await
        .expect_err("repository failure should bubble up");

    assert!(matches!(
        error,
        AppError::DatabaseError(sqlx::Error::RowNotFound)
    ));
    assert_eq!(storage_entry_count(service).await, 0);
}

#[tokio::test]
async fn test_upload_from_file_cleans_temp_file_on_processing_failure() {
    let context = setup_service().await;
    let service = &context.service;
    let user_id = Uuid::new_v4();
    let temp_path = context._temp_dir.path().join("pending-upload.png");

    tokio::fs::write(&temp_path, b"not-an-image")
        .await
        .expect("temp upload should be written");

    let result = service
        .upload_image_from_file(
            user_id,
            "alice",
            "pending-upload.png".to_string(),
            temp_path.clone(),
            Some("image/png".to_string()),
        )
        .await;

    assert!(result.is_err(), "invalid payload should fail processing");
    assert!(
        !tokio::fs::try_exists(&temp_path)
            .await
            .expect("temp path existence check should succeed"),
        "temp upload file should always be cleaned up"
    );
}

#[tokio::test]
async fn test_hard_delete_preserves_shared_file_until_last_reference_is_removed() {
    let context = setup_service().await;
    let service = &context.service;
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();
    let payload = sample_png_bytes();

    let first = service
        .upload_image(
            user_a,
            "alice",
            "shared.png".to_string(),
            payload.clone(),
            Some("image/png".to_string()),
        )
        .await
        .expect("first upload should succeed");
    let second = service
        .upload_image(
            user_b,
            "bob",
            "shared.png".to_string(),
            payload,
            Some("image/png".to_string()),
        )
        .await
        .expect("second upload should succeed");

    assert_eq!(first.filename, second.filename);
    assert_eq!(first.thumbnail, second.thumbnail);
    let file_path = std::path::Path::new(&service.config.storage.path).join(&first.filename);
    let thumbnail_path = std::path::Path::new(&service.config.storage.path).join(
        first
            .thumbnail
            .clone()
            .expect("uploaded image should have thumbnail"),
    );
    assert!(tokio::fs::try_exists(&file_path).await.unwrap());
    assert!(tokio::fs::try_exists(&thumbnail_path).await.unwrap());

    service
        .delete_images(&[first.id], user_a)
        .await
        .expect("first hard delete should succeed");

    assert!(
        tokio::fs::try_exists(&file_path).await.unwrap(),
        "shared file should remain while another record still references it"
    );
    assert!(
        tokio::fs::try_exists(&thumbnail_path).await.unwrap(),
        "shared thumbnail should remain while another record still references it"
    );

    service
        .delete_images(&[second.id], user_b)
        .await
        .expect("second hard delete should succeed");

    assert!(
        !tokio::fs::try_exists(&file_path).await.unwrap(),
        "shared file should be removed after last reference is deleted"
    );
    assert!(
        !tokio::fs::try_exists(&thumbnail_path).await.unwrap(),
        "shared thumbnail should be removed after last reference is deleted"
    );
}

#[tokio::test]
async fn test_delete_keeps_storage_when_repository_delete_fails() {
    let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").unwrap();
    let context = setup_service_with_repository(
        DatabasePool::Postgres(pool),
        FaultyImageRepository::fail_hard_delete(),
    )
    .await;
    let service = &context.service;
    let user_id = Uuid::new_v4();
    let image = build_image(
        Uuid::new_v4(),
        user_id,
        "delete-failure.jpg",
        &valid_hash(11),
        Utc::now(),
    );
    let file_path = std::path::Path::new(&service.config.storage.path).join(&image.filename);

    service
        .image_repository
        .inner
        .create_image(&image)
        .await
        .expect("image should be inserted into mock repository");
    tokio::fs::create_dir_all(&service.config.storage.path)
        .await
        .expect("storage root should be created");
    tokio::fs::write(&file_path, b"payload")
        .await
        .expect("physical object should exist before delete");

    let error = service
        .delete_images(&[image.id], user_id)
        .await
        .expect_err("repository delete failure should stop hard delete");

    assert!(matches!(
        error,
        AppError::DatabaseError(sqlx::Error::RowNotFound)
    ));
    assert!(
        tokio::fs::try_exists(&file_path)
            .await
            .expect("file existence check should succeed"),
        "physical object must remain when metadata delete fails"
    );
}
