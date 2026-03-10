use super::*;
use crate::config::Config;
use crate::db::DatabasePool;
use crate::domain::image::mock_repository::MockImageRepository;
use crate::image_processor::ImageProcessor;
use crate::runtime_settings::{RuntimeSettings, StorageBackend};
use crate::storage_backend::StorageManager;
use std::sync::Arc;
use tempfile::TempDir;

struct TestServiceContext {
    service: ImageDomainService<MockImageRepository>,
    _temp_dir: TempDir,
}

async fn setup_service() -> TestServiceContext {
    let mut config = Config::default();
    config.storage.enable_file_check = false;
    let image_processor = ImageProcessor::new(1920, 1080, 80);

    let redis = None;
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let local_storage_path = temp_dir.path().join("images");
    config.storage.path = local_storage_path.to_string_lossy().into_owned();

    let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").unwrap();
    let storage_manager = Arc::new(StorageManager::new(RuntimeSettings {
        site_name: "Vansour Image".to_string(),
        storage_backend: StorageBackend::Local,
        local_storage_path: config.storage.path.clone(),
        mail_enabled: false,
        mail_smtp_host: "smtp.example.com".to_string(),
        mail_smtp_port: 587,
        mail_smtp_user: None,
        mail_smtp_password: None,
        mail_from_email: "noreply@example.com".to_string(),
        mail_from_name: "Vansour Image".to_string(),
        mail_link_base_url: "https://img.example.com".to_string(),
        s3_endpoint: None,
        s3_region: None,
        s3_bucket: None,
        s3_prefix: None,
        s3_access_key: None,
        s3_secret_key: None,
        s3_force_path_style: true,
    }));
    let dependencies = ImageDomainServiceDependencies::new(
        DatabasePool::Postgres(pool),
        redis,
        config,
        image_processor,
        storage_manager,
    );

    TestServiceContext {
        service: ImageDomainService::new(dependencies, MockImageRepository::new()),
        _temp_dir: temp_dir,
    }
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
    deleted_at: Option<chrono::DateTime<Utc>>,
) -> Image {
    Image {
        id,
        user_id,
        category_id: None,
        filename: filename.to_string(),
        thumbnail: None,
        original_filename: None,
        size: 100,
        hash: hash.to_string(),
        format: "jpg".to_string(),
        views: 0,
        status: "active".to_string(),
        expires_at: None,
        deleted_at,
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
async fn test_increment_views() {
    let service = setup_service().await.service;
    let user_id = Uuid::new_v4();
    let image_id = Uuid::new_v4();

    let image = Image {
        id: image_id,
        user_id,
        category_id: None,
        filename: "test.jpg".to_string(),
        thumbnail: None,
        original_filename: None,
        size: 100,
        hash: "hash1".to_string(),
        format: "jpg".to_string(),
        views: 0,
        status: "active".to_string(),
        expires_at: None,
        deleted_at: None,
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
        "hash-newer",
        Utc::now(),
        None,
    );
    let older = build_image(
        Uuid::new_v4(),
        user_id,
        "older.jpg",
        "hash-older",
        Utc::now() - chrono::Duration::days(1),
        None,
    );

    service.image_repository.create_image(&older).await.unwrap();
    service.image_repository.create_image(&newer).await.unwrap();

    let page = service.get_images(user_id, 1, 20, None).await.unwrap();

    assert_eq!(page.data.len(), 2);
    assert_eq!(page.data[0].id, newer.id);
    assert_eq!(page.data[1].id, older.id);
}

#[tokio::test]
async fn test_list_deleted_images_ordered_desc() {
    let service = setup_service().await.service;
    let user_id = Uuid::new_v4();
    let recently_deleted = build_image(
        Uuid::new_v4(),
        user_id,
        "recent.jpg",
        "hash-del-1",
        Utc::now() - chrono::Duration::days(2),
        Some(Utc::now()),
    );
    let older_deleted = build_image(
        Uuid::new_v4(),
        user_id,
        "older.jpg",
        "hash-del-2",
        Utc::now() - chrono::Duration::days(5),
        Some(Utc::now() - chrono::Duration::days(1)),
    );

    service
        .image_repository
        .create_image(&older_deleted)
        .await
        .unwrap();
    service
        .image_repository
        .create_image(&recently_deleted)
        .await
        .unwrap();

    let page = service
        .get_deleted_images_paginated(user_id, 1, 20)
        .await
        .unwrap();

    assert_eq!(page.data.len(), 2);
    assert_eq!(page.data[0].id, recently_deleted.id);
    assert_eq!(page.data[1].id, older_deleted.id);
    assert_eq!(page.total, 2);
}

#[tokio::test]
async fn test_bulk_delete_soft_marks_images_deleted() {
    let service = setup_service().await.service;
    let user_id = Uuid::new_v4();
    let image_a = build_image(
        Uuid::new_v4(),
        user_id,
        "a.jpg",
        "hash-bulk-a",
        Utc::now(),
        None,
    );
    let image_b = build_image(
        Uuid::new_v4(),
        user_id,
        "b.jpg",
        "hash-bulk-b",
        Utc::now() - chrono::Duration::minutes(5),
        None,
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
        .delete_images(&[image_a.id, image_b.id], user_id, false)
        .await
        .unwrap();

    let deleted_images = service
        .get_deleted_images_paginated(user_id, 1, 20)
        .await
        .unwrap();
    assert_eq!(deleted_images.total, 2);
}

#[tokio::test]
async fn test_restore_images_reactivates_deleted_entries() {
    let service = setup_service().await.service;
    let user_id = Uuid::new_v4();
    let deleted_image = build_image(
        Uuid::new_v4(),
        user_id,
        "restore.jpg",
        "hash-restore",
        Utc::now() - chrono::Duration::hours(3),
        Some(Utc::now()),
    );

    service
        .image_repository
        .create_image(&deleted_image)
        .await
        .unwrap();

    service
        .restore_images(&[deleted_image.id], user_id)
        .await
        .unwrap();

    let active = service.get_images(user_id, 1, 20, None).await.unwrap();
    assert_eq!(active.total, 1);
    assert!(active.data.iter().all(|image| image.deleted_at.is_none()));
}

#[tokio::test]
async fn test_restore_images_rejects_invalid_key() {
    let service = setup_service().await.service;
    let user_id = Uuid::new_v4();

    let result = service
        .restore_images_by_keys(&["invalid-key".to_string()], user_id)
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
    let file_path = std::path::Path::new(&service.config.storage.path).join(&first.filename);
    assert!(tokio::fs::try_exists(&file_path).await.unwrap());

    service
        .delete_images(&[first.id], user_a, true)
        .await
        .expect("first hard delete should succeed");

    assert!(
        tokio::fs::try_exists(&file_path).await.unwrap(),
        "shared file should remain while another record still references it"
    );

    service
        .delete_images(&[second.id], user_b, true)
        .await
        .expect("second hard delete should succeed");

    assert!(
        !tokio::fs::try_exists(&file_path).await.unwrap(),
        "shared file should be removed after last reference is deleted"
    );
}
