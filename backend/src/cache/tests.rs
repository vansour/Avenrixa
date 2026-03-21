use super::*;
use uuid::Uuid;

#[test]
fn test_image_cache_list() {
    let user_id = Uuid::new_v4();
    let cursor = Some("cursor-token");
    let limit = 20;

    let key = ImageCache::list(user_id, cursor, limit);

    assert!(key.contains(&user_id.to_string()));
    assert!(key.contains(cursor.unwrap()));
    assert!(key.contains(&limit.to_string()));
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

    let key1 = ImageCache::list(user_id, None, 20);
    let key2 = ImageCache::list(user_id, Some("next"), 20);

    assert_ne!(key1, key2);
}

#[test]
fn test_hash_cache_user_existing_info_key_format() {
    let user_id = Uuid::new_v4();
    let key = HashCache::existing_info("abc123", "user", user_id);

    assert!(key.starts_with("hash:info:user:"));
    assert!(key.ends_with(&format!(":{}", user_id)));
}
