use super::*;
use uuid::Uuid;

#[test]
fn test_image_cache_list_without_category() {
    let user_id = Uuid::new_v4();
    let page = 1;
    let page_size = 20;

    let key = ImageCache::list(user_id, page, page_size, None);

    assert!(key.contains(&user_id.to_string()));
    assert!(key.contains(&page.to_string()));
    assert!(key.contains(&page_size.to_string()));
    assert!(key.starts_with("images:list:"));
}

#[test]
fn test_image_cache_list_with_category() {
    let user_id = Uuid::new_v4();
    let page = 2;
    let page_size = 50;
    let category_id = Some(Uuid::new_v4());

    let key = ImageCache::list(user_id, page, page_size, category_id);

    assert!(key.contains(&user_id.to_string()));
    if let Some(category_id) = category_id {
        assert!(key.contains(&category_id.to_string()));
    }
    assert!(key.contains(&page.to_string()));
    assert!(key.contains(&page_size.to_string()));
}

#[test]
fn test_image_cache_categories() {
    let user_id = Uuid::new_v4();
    let key = ImageCache::categories(user_id);
    assert_eq!(key, format!("categories:list:{}", user_id));
}

#[test]
fn test_cache_key_prefix() {
    let cache = Cache::new("test_prefix");
    let key = cache.key("test_key");
    assert_eq!(key, "test_prefixtest_key");
}

#[test]
fn test_cache_key_empty_prefix() {
    let cache = Cache::new("");
    let key = cache.key("test_key");
    assert_eq!(key, "test_key");
}

#[test]
fn test_image_cache_different_pages_and_categories() {
    let user_id = Uuid::new_v4();
    let category_id = Some(Uuid::new_v4());

    let key1 = ImageCache::list(user_id, 1, 20, None);
    let key2 = ImageCache::list(user_id, 2, 20, None);
    let key3 = ImageCache::list(user_id, 1, 20, category_id);

    assert_ne!(key1, key2);
    assert_ne!(key1, key3);
    assert_ne!(key2, key3);
}

#[test]
fn test_hash_cache_user_existing_info_invalidate_pattern() {
    let user_id = Uuid::new_v4();
    let key = HashCache::existing_info("abc123", "user", user_id);
    let pattern = HashCache::user_existing_info_invalidate(user_id);

    assert_eq!(pattern, format!("hash:info:user:*:{}", user_id));
    assert!(key.starts_with("hash:info:user:"));
    assert!(key.ends_with(&format!(":{}", user_id)));
}
