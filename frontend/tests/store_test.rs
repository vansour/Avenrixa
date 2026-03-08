#[test]
fn test_auth_store_new() {
    use vansour_image_frontend::store::auth::AuthStore;

    let store = AuthStore::new();
    assert!(store.user_is_none());
    assert!(!store.is_authenticated());
}

#[test]
fn test_auth_store_login_logout() {
    use vansour_image_frontend::store::auth::AuthStore;
    use vansour_image_frontend::types::api::UserResponse;
    use uuid::Uuid;
    use chrono::Utc;

    let store = AuthStore::new();

    // 测试登录
    let user = UserResponse {
        id: Uuid::new_v4(),
        username: "test_user".to_string(),
        role: "user".to_string(),
        created_at: Utc::now(),
    };
    let token = "test_token".to_string();

    store.login(user.clone(), token);

    assert!(store.is_authenticated());
    assert!(store.user_is_some());
    assert_eq!(store.user_as_ref().unwrap().username, "test_user");

    // 测试登出
    store.logout();
    assert!(!store.is_authenticated());
    assert!(store.user_is_none());
}

#[test]
fn test_image_store_new() {
    use vansour_image_frontend::store::images::ImageStore;

    let store = ImageStore::new();
    assert!(store.images_is_empty());
    assert_eq!(store.current_page(), 1);
    assert_eq!(store.total_items(), 0);
    assert!(store.has_more());
    assert!(!store.is_loading());
}

#[test]
fn test_image_store_add_images() {
    use vansour_image_frontend::store::images::ImageStore;
    use vansour_image_frontend::types::models::ImageItem;
    use chrono::Utc;

    let store = ImageStore::new();
    let image = ImageItem {
        id: "1".to_string(),
        filename: "test.jpg".to_string(),
        original_filename: Some("test_original.jpg".to_string()),
        size: 1024,
        format: "jpg".to_string(),
        created_at: Utc::now(),
        thumbnail_url: None,
        url: "http://example.com/test.jpg".to_string(),
    };

    store.add_images(vec![image.clone()]);
    let images = store.images();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].id, "1");
}

#[test]
fn test_ui_store_new() {
    use vansour_image_frontend::store::ui::UIStore;

    let store = UIStore::new();
    assert!(!store.sidebar_open());
    assert!(store.toast_message_is_none());
}

#[test]
fn test_ui_store_toggle_sidebar() {
    use vansour_image_frontend::store::ui::UIStore;

    let store = UIStore::new();
    assert!(!store.sidebar_open());

    store.toggle_sidebar();
    assert!(store.sidebar_open());

    store.toggle_sidebar();
    assert!(!store.sidebar_open());
}
