pub(super) const IMAGE_SELECT_COLUMNS: &str = concat!(
    "id, user_id, filename, thumbnail, ",
    "size, hash, format, views, status, expires_at, created_at"
);

pub(super) const MEDIA_BLOB_SELECT_COLUMNS: &str = concat!(
    "storage_key, media_kind, content_hash, ",
    "ref_count, status, created_at, updated_at"
);
