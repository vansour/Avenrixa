use super::*;
use uuid::Uuid;

#[test]
fn test_image_cache_list() {
    let user_id = Uuid::new_v4();
    let page = 1;
    let page_size = 20;

    let key = ImageCache::list(user_id, page, page_size);

    assert!(key.contains(&user_id.to_string()));
    assert!(key.contains(&page.to_string()));
    assert!(key.contains(&page_size.to_string()));
    assert!(key.starts_with("images:list:"));
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
fn test_image_cache_different_pages() {
    let user_id = Uuid::new_v4();

    let key1 = ImageCache::list(user_id, 1, 20);
    let key2 = ImageCache::list(user_id, 2, 20);

    assert_ne!(key1, key2);
}

#[test]
fn test_hash_cache_user_existing_info_key_format() {
    let user_id = Uuid::new_v4();
    let key = HashCache::existing_info("abc123", "user", user_id);

    assert!(key.starts_with("hash:info:user:"));
    assert!(key.ends_with(&format!(":{}", user_id)));
}
