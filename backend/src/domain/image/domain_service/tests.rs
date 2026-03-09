use super::*;
use crate::config::Config;
use crate::domain::image::mock_repository::{MockCategoryRepository, MockImageRepository};
use crate::file_queue::FileSaveQueue;
use crate::image_processor::ImageProcessor;
use crate::runtime_settings::RuntimeSettingsService;
use crate::storage_backend::StorageManager;
use std::sync::Arc;

async fn setup_service() -> ImageDomainService<MockImageRepository, MockCategoryRepository> {
    let mut config = Config::default();
    config.storage.enable_file_check = false;
    let image_processor = ImageProcessor::new(1920, 1080, 200, 80);

    let redis = None;
    let file_save_queue = Arc::new(FileSaveQueue::new_mock());

    let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").unwrap();
    let runtime_settings = Arc::new(RuntimeSettingsService::new(pool.clone(), &config));
    let storage_manager = Arc::new(StorageManager::new(runtime_settings));
    let dependencies = ImageDomainServiceDependencies::new(
        pool,
        redis,
        config,
        image_processor,
        file_save_queue,
        storage_manager,
    );

    ImageDomainService::new(
        dependencies,
        MockImageRepository::new(),
        MockCategoryRepository::new(),
    )
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
    let service = setup_service().await;
    let user_id = Uuid::new_v4();
    let image_id = Uuid::new_v4();
    let result = service.get_image_by_id(image_id, user_id).await;
    assert!(matches!(result, Err(AppError::ImageNotFound)));
}

#[tokio::test]
async fn test_increment_views() {
    let service = setup_service().await;
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
    let service = setup_service().await;
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

    let page = service
        .get_images(user_id, 1, 20, None, None)
        .await
        .unwrap();

    assert_eq!(page.data.len(), 2);
    assert_eq!(page.data[0].id, newer.id);
    assert_eq!(page.data[1].id, older.id);
}

#[tokio::test]
async fn test_list_deleted_images_ordered_desc() {
    let service = setup_service().await;
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
    let service = setup_service().await;
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
    let service = setup_service().await;
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

    let active = service
        .get_images(user_id, 1, 20, None, None)
        .await
        .unwrap();
    assert_eq!(active.total, 1);
    assert!(active.data.iter().all(|image| image.deleted_at.is_none()));
}

#[tokio::test]
async fn test_restore_images_rejects_invalid_key() {
    let service = setup_service().await;
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
